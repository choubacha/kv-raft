use super::{network, proto, public::Command, storage::KeyValue, Message};
use futures::sync::mpsc;
use futures::Stream;
use protobuf::parse_from_bytes;
use public;
use raft::{self, prelude::*};
use std::collections::HashMap;
use std::num::Wrapping;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio;
use tokio::timer::Interval;

pub struct Handle {
    handle: JoinHandle<()>,
    tx: mpsc::Sender<Message>,
}

impl Handle {
    pub fn join(self) {
        self.handle.join().expect("Failed to join db server");
    }

    pub fn channel(&self) -> mpsc::Sender<Message> {
        self.tx.clone()
    }
}

struct Callbacks {
    commands: HashMap<Wrapping<u64>, Command>,
    curr_id: Wrapping<u64>,
}

impl Callbacks {
    fn new() -> Callbacks {
        Callbacks {
            commands: HashMap::new(),
            curr_id: Wrapping(0),
        }
    }

    fn store_delete(&mut self, command: Command) -> proto::Entry {
        self.curr_id += Wrapping(1);

        let key = {
            let delete = command.request().get_delete();
            delete.get_key().to_string()
        };

        self.commands.insert(self.curr_id, command);

        let mut entry = proto::Entry::new();
        entry.set_id(self.curr_id.0);
        entry.set_key(key);
        entry.set_kind(proto::EntryKind::DELETE);
        entry
    }

    fn store_set(&mut self, command: Command) -> proto::Entry {
        self.curr_id += Wrapping(1);

        let (key, value) = {
            let set = command.request().get_set();
            (set.get_key().to_string(), set.get_value().to_string())
        };

        self.commands.insert(self.curr_id, command);

        let mut entry = proto::Entry::new();
        entry.set_id(self.curr_id.0);
        entry.set_key(key);
        entry.set_value(value);
        entry.set_kind(proto::EntryKind::SET);
        entry
    }

    fn get(&mut self, id: u64) -> Option<Command> {
        self.commands.remove(&Wrapping(id))
    }
}

#[cfg(test)]
mod callback_tests {
    use super::*;

    #[test]
    fn test_set_command() {
        let (tx, _) = mpsc::channel(1024);
        let cmd = Command::new(tx, public::set_request("hello", "world"));
        let mut cbs = Callbacks::new();
        let entry = cbs.store_set(cmd);
        assert_eq!(entry.id, 1);
        let cmd = cbs.get(entry.id).unwrap();

        let entry = cbs.store_set(cmd);
        assert_eq!(entry.id, 2);
    }

    #[test]
    fn test_delete_command() {
        let (tx, _) = mpsc::channel(1024);
        let cmd = Command::new(tx, public::delete_request("hello"));
        let mut cbs = Callbacks::new();
        let entry = cbs.store_delete(cmd);
        assert_eq!(entry.id, 1);
        let cmd = cbs.get(entry.id).unwrap();

        let entry = cbs.store_delete(cmd);
        assert_eq!(entry.id, 2);
    }
}

/// The database does not communicate on a network but instead uses
/// a set of channels to communicate.
pub struct Db {
    node: RawNode<KeyValue>,
    network: network::Handle,
    callbacks: Callbacks,
}

impl Db {
    pub fn new(id: u64, file: &str, network: network::Handle) -> Db {
        let config = Config {
            id,
            heartbeat_tick: 1,
            election_tick: 10,
            max_inflight_msgs: 1024,
            pre_vote: true,
            ..Config::default()
        };
        config.validate().unwrap();

        let node = RawNode::new(&config, KeyValue::new(file), network.peers()).unwrap();
        let callbacks = Callbacks::new();

        Db {
            network,
            node,
            callbacks,
        }
    }

    pub fn start(mut self) -> Handle {
        let (tx, rx) = mpsc::channel(1024);
        let handle = thread::spawn(move || {
            const HEARTBEAT: Duration = Duration::from_millis(500);

            let timer = Interval::new(Instant::now(), HEARTBEAT)
                .map(|_| Message::Timeout)
                .map_err(|_| ())
                .select(rx.map_err(|e| println!("error: {:?}", e)))
                .for_each(move |msg| {
                    // The db is receiving commands/messages here which are then
                    // worked on and possibly generate other return values.
                    match msg {
                        Message::Timeout => {
                            self.node.tick();
                        }
                        Message::Cmd(command) => self.handle(command),
                        Message::Raft(message) => {
                            println!("Received raft message...");
                            self.node.step(message).unwrap();
                        }
                        Message::Ping => {
                            println!("PING");
                        }
                        Message::Stop => {
                            println!("requested to stop");
                            return Err(());
                        }
                    }

                    self.check_ready();

                    Ok(())
                });
            tokio::run(timer);
        });
        Handle { handle, tx }
    }

    fn handle(&mut self, command: Command) {
        if command.request().has_ping() {
            self.handle_ping(command);
        } else if command.request().has_get() {
            self.handle_get(command);
        } else if command.request().has_scan() {
            self.handle_scan(command);
        } else if command.request().has_delete() {
            self.handle_delete(command);
        } else if command.request().has_set() {
            self.handle_set(command);
        }
    }

    fn handle_get(&self, command: Command) {
        let value = {
            let get = command.request().get_get();
            self.node.get_store().rl().get(get.get_key())
        };
        command.reply(public::get_response(value));
    }

    fn handle_scan(&self, command: Command) {
        let keys = self.node.get_store().rl().scan();
        command.reply(public::scan_response(keys));
    }

    fn handle_set(&mut self, command: Command) {
        use protobuf::Message;

        let entry = self.callbacks.store_set(command);

        self.node
            .propose(Vec::new(), entry.write_to_bytes().unwrap())
            .unwrap();
    }

    fn handle_delete(&mut self, command: Command) {
        use protobuf::Message;

        let entry = self.callbacks.store_delete(command);

        self.node
            .propose(Vec::new(), entry.write_to_bytes().unwrap())
            .unwrap();
    }

    fn handle_ping(&self, command: Command) {
        command.reply(public::ping_response());
    }

    fn check_ready(&mut self) {
        if !self.node.has_ready() {
            return;
        }

        println!("Raft is ready...");

        // The Raft is ready, we can do something now.
        let mut ready = self.node.ready();

        let is_leader = self.is_leader();

        // Leaders should send messages right away
        if is_leader {
            let msgs = ready.messages.drain(..);
            for msg in msgs {
                ::tokio::spawn(self.network.send(msg.to, msg));
            }
        }

        if !raft::is_empty_snap(&ready.snapshot) {
            println!("Applying snap shot");
            self.node
                .mut_store()
                .wl()
                .apply_snapshot(ready.snapshot.clone())
                .unwrap();
        }

        if !ready.entries.is_empty() {
            println!("Saving entries...");
            self.node.mut_store().wl().append(&ready.entries).unwrap();
        }

        if let Some(ref hs) = ready.hs {
            println!("Save hardstate...");

            // Raft HardState changed, and we need to persist it.
            self.node.mut_store().wl().set_hardstate(hs.clone());
        }

        // Followers should reply messages
        if !is_leader {
            let msgs = ready.messages.drain(..);
            for msg in msgs {
                ::tokio::spawn(self.network.send(msg.to, msg));
            }
        }

        if let Some(committed_entries) = ready.committed_entries.take() {
            let mut last_apply_index = 0;
            let mut conf_state: Option<ConfState> = None;
            for entry in committed_entries {
                println!("Updating state based on entry...");

                // Mostly, you need to save the last apply index to resume applying
                // after restart. Here we just ignore this because we use a Memory storage.
                last_apply_index = entry.get_index();

                let data = entry.get_data();

                if data.is_empty() {
                    // Emtpy entry, when the peer becomes Leader it will send an empty entry.
                    continue;
                }

                // Modify the state and reply
                match entry.get_entry_type() {
                    EntryType::EntryNormal => {
                        let entry = parse_from_bytes::<proto::Entry>(data).expect("Valid protobuf");

                        let response = match entry.kind {
                            proto::EntryKind::SET => {
                                self.node.mut_store().wl().set(&entry.key, &entry.value);
                                public::set_response()
                            }
                            proto::EntryKind::DELETE => public::delete_response(
                                self.node.mut_store().wl().delete(&entry.key),
                            ),
                        };

                        if let Some(cmd) = self.callbacks.get(entry.id) {
                            cmd.reply(response);
                        }
                    }
                    EntryType::EntryConfChange => {
                        let cc = parse_from_bytes::<ConfChange>(data).expect("Valid protobuf");
                        conf_state = Some(self.node.apply_conf_change(&cc));
                        println!("Updated state: {:?}", conf_state);
                    }
                }
            }

            let mut mem = self.node.mut_store().wl();
            mem.create_snapshot(last_apply_index, conf_state);

            // Now that we have a snapshot, let's compact out the rest
            // if last_apply_index > 0 {
            //    mem.compact(last_apply_index - 1).unwrap();
            //}
        }
        self.node.advance(ready);
    }

    fn is_leader(&self) -> bool {
        self.node.raft.leader_id == self.node.raft.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{Future, Sink};
    use server::public::Command;

    #[test]
    fn test_start_and_stop() {
        let network = network::start();

        let db = Db::new(1, "/tmp/data", network);
        let handle = db.start();
        let channel = handle.channel();

        tokio::run({
            channel
                .clone()
                .send(Message::Ping)
                .then(move |_| channel.clone().send(Message::Stop))
                .then(|_| Ok(()))
        });
        handle.join();
    }
}
