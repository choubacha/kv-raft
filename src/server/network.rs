use super::proto;
use codec::Proto;
use futures::prelude::*;
use futures::sync::mpsc;
use raft;
use raft::raw_node::Peer as RaftPeer;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::thread::{self, JoinHandle};
use tokio;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::FramedWrite;

pub fn start() -> Handle {
    let (tx, rx) = mpsc::channel(1024);

    let handle = thread::spawn(move || Network::new().listen(rx));

    Handle {
        tx,
        handle,
        ids: vec![],
    }
}

#[derive(Debug)]
pub struct Handle {
    tx: mpsc::Sender<Cmd>,
    handle: JoinHandle<()>,
    ids: Vec<u64>,
}

impl Handle {
    pub fn send(&self, id: u64, msg: raft::eraftpb::Message) -> impl Future<Item = (), Error = ()> {
        self.tx
            .clone()
            .send(Cmd::raft(id, msg))
            .map(|_| ())
            .map_err(|e| println!("Error when queuing message to network: {:?}", e))
    }

    pub fn peers(&self) -> Vec<RaftPeer> {
        self.ids
            .iter()
            .map(|id| RaftPeer {
                id: *id,
                context: None,
            })
            .collect()
    }

    pub fn add_peer(&mut self, peer: &proto::Peer) -> impl Future<Item = (), Error = ()> {
        self.add(peer.get_id(), peer.get_addr().to_string())
    }

    pub fn add(&mut self, id: u64, addr: String) -> impl Future<Item = (), Error = ()> {
        assert!(id > 0);
        self.ids.push(id);
        self.tx
            .clone()
            .send(Cmd::add(id, addr))
            .map(|_| ())
            .map_err(|e| println!("Error when queuing node add: {:?}", e))
    }
}

/// The network is a set of known peers that are used to communicated
/// outwardly. The network has a single input but routes it to the
/// correct peer.
#[derive(Debug)]
struct Network {
    peers: HashMap<u64, Peer>,
}

#[derive(Debug)]
struct Peer {
    tx: mpsc::Sender<raft::eraftpb::Message>,
    id: u64,
}

#[derive(Debug, Clone)]
enum Kind {
    Add(String),
    Raft(raft::eraftpb::Message),
}

#[derive(Debug, Clone)]
struct Cmd {
    id: u64,
    kind: Kind,
}

impl Cmd {
    fn add(id: u64, addr: String) -> Self {
        Cmd {
            id,
            kind: Kind::Add(addr),
        }
    }

    fn raft(id: u64, msg: raft::eraftpb::Message) -> Self {
        Cmd {
            id,
            kind: Kind::Raft(msg),
        }
    }
}

impl Network {
    fn new() -> Network {
        Network {
            peers: HashMap::new(),
        }
    }

    fn listen(mut self, rx: mpsc::Receiver<Cmd>) {
        let network = rx.for_each(move |cmd| {
            match cmd.kind {
                Kind::Add(addr) => self.add(cmd.id, addr),
                Kind::Raft(msg) => self.send(cmd.id, msg),
            }
            Ok(())
        });
        tokio::run(network);
    }

    fn add(&mut self, id: u64, addr: String) {
        let (tx, rx) = mpsc::channel(1024);
        self.peers.insert(id, Peer { tx, id });

        tokio::spawn({
            rx.for_each(move |msg| {
                if let Ok(Some(addr)) = addr.to_socket_addrs().map(|mut i| i.next()) {
                    tokio::spawn(
                        TcpStream::connect(&addr)
                            .map_err(|e| println!("Error connecting to peer: {:?}", e))
                            .and_then(move |sock| {
                                let (_, sink) = sock.split();

                                let sink =
                                    FramedWrite::new(sink, Proto::<raft::eraftpb::Message>::new());
                                sink.send(msg)
                                    .map(|_| ())
                                    .map_err(|e| println!("Error sending message to peer: {:?}", e))
                            }),
                    );
                }
                Ok(())
            })
        });
    }

    fn send(&self, id: u64, msg: raft::eraftpb::Message) {
        if let Some(peer) = self.peers.get(&id) {
            tokio::spawn({
                peer.tx
                    .clone()
                    .send(msg)
                    .map(|_| ())
                    .map_err(|e| println!("Error when sending message over peer channel: {:?}", e))
            });
        }
    }
}
