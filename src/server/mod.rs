use futures::sync::mpsc::Sender;
use futures::Future;
use public::{self, Request, Response};
use raft::{self, raw_node::RawNode, storage::MemStorage, Config};
use std::net::SocketAddr;
use std::thread::{self, JoinHandle};
use tokio;

mod db;

#[derive(Debug, Clone)]
pub enum Message {
    Timeout,
    Msg(Request),
    Raft(raft::eraftpb::Message),
    Stop,
}

pub struct Server {
    client: ClientHandle,
    peer: PeerHandle,
    db: db::Handle,
}

struct ClientListener {}

struct ClientHandle(JoinHandle<()>);

impl ClientHandle {
    fn join(self) {
        self.0.join();
    }
}

impl ClientListener {
    fn new() -> ClientListener {
        ClientListener {}
    }
}

struct ConnectionHandle {
    sender: Sender<Response>,
}

/// The peer listener is a tcp listener that listens for raft messages
/// on the peer network and then forwards them to the db.
struct PeerListener;

struct Peer {
    id: u64,
    addr: SocketAddr,
}

struct PeerHandle(JoinHandle<()>);

impl PeerHandle {
    fn join(self) {
        self.0.join();
    }
}

impl Server {
    fn start() {}
}
