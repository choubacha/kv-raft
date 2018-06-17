use super::Message;
use futures::sync::mpsc::{channel, Receiver, Sender};
use futures::{Future, Sink, Stream};
use public::{self, Request, Response};
use raft::{self, raw_node::RawNode, storage::MemStorage, Config};
use std::collections::HashMap;
use std::sync::RwLock;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio;
use tokio::timer::Interval;

pub struct Handle {
    handle: JoinHandle<()>,
    sender: Sender<Message>,
}

impl Handle {
    pub fn join(self) {
        self.handle.join().expect("Failed to join db server");
    }

    pub fn channel(&self) -> Sender<Message> {
        self.sender.clone()
    }
}

/// The database does not communicate on a network but instead uses
/// a set of channels to communicate.
struct Db {
    state: RwLock<HashMap<String, String>>,
    raft: RawNode<MemStorage>,
}

impl Db {
    fn new() -> Db {
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

    fn start(self) -> Handle {
        let (sender, receiver) = channel(1024);
        let handle = thread::spawn(move || {
            const HEARTBEAT: Duration = Duration::from_millis(100);

            let timer = Interval::new(Instant::now() + HEARTBEAT, HEARTBEAT)
                .map(|_| Message::Timeout)
                .map_err(|_| ())
                .select(receiver.map_err(|e| println!("error: {:?}", e)))
                .for_each(move |msg| {
                    // The db is receiving commands/messages here which are then
                    // worked on and possibly generate other return values.
                    match msg {
                        Message::Timeout => {
                            println!("timed out");
                        }
                        Message::Msg(request) => {
                            println!("request! {:?}", request);
                            println!("state: {:?}", &self.state);
                        }
                        Message::Raft(_) => {}
                        Message::Stop => {
                            println!("requested to stop");
                            return Err(());
                        }
                    }
                    Ok(())
                });
            tokio::run(timer);
        });
        Handle { handle, sender }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_and_stop() {
        let db = Db::new();
        let handle = db.start();
        let channel = handle.channel();
        tokio::run({
            channel
                .clone()
                .send(Message::Msg(public::get_request("hello")))
                .then(move |_| channel.clone().send(Message::Stop))
                .then(|_| Ok(()))
        });
        handle.join();
    }
}
