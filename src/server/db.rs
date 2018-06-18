use super::Message;
use futures::sync::mpsc;
use futures::Stream;
use raft::{raw_node::RawNode, storage::MemStorage, Config};
use std::collections::HashMap;
use std::sync::RwLock;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio;
use tokio::timer::Interval;
use public;

type Tx = mpsc::Sender<Message>;
type Rx = mpsc::Receiver<Message>;

pub struct Handle {
    handle: JoinHandle<()>,
    tx: Tx,
}

impl Handle {
    pub fn join(self) {
        self.handle.join().expect("Failed to join db server");
    }

    pub fn channel(&self) -> Tx {
        self.tx.clone()
    }
}

/// The database does not communicate on a network but instead uses
/// a set of channels to communicate.
pub struct Db {
    state: RwLock<HashMap<String, String>>,
    raft: RawNode<MemStorage>,
}

impl Db {
    pub fn new() -> Db {
        let state = RwLock::new(HashMap::new());
        let config = Config {
            id: 1,
            heartbeat_tick: 3,
            election_tick: 30,
            max_inflight_msgs: 1024,
            ..Config::default()
        };
        let raft = RawNode::new(&config, MemStorage::default(), Vec::new()).unwrap();
        Db { state, raft }
    }

    pub fn start(self) -> Handle {
        let (tx, rx) = mpsc::channel(1024);
        let handle = thread::spawn(move || {
            const HEARTBEAT: Duration = Duration::from_millis(100);

            let timer = Interval::new(Instant::now() + HEARTBEAT, HEARTBEAT)
                .map(|_| Message::Timeout)
                .map_err(|_| ())
                .select(rx.map_err(|e| println!("error: {:?}", e)))
                .for_each(move |msg| {
                    // The db is receiving commands/messages here which are then
                    // worked on and possibly generate other return values.
                    match msg {
                        Message::Timeout => {
                            println!("timed out");
                        }
                        Message::Cmd(command) => {
                            println!("command: {:?}", command);

                            if command.request().has_ping() {
                                command.reply(public::ping_response());
                            }

                            println!("state: {:?}", &self.state);
                        }
                        Message::Raft(_) => {}
                        Message::Ping => {
                            println!("PING");
                        }
                        Message::Stop => {
                            println!("requested to stop");
                            return Err(());
                        }
                    }
                    Ok(())
                });
            tokio::run(timer);
        });
        Handle { handle, tx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use server::public::Command;
    use futures::{Sink, Future};

    #[test]
    fn test_start_and_stop() {
        let db = Db::new();
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
