use codec::Proto;
use futures::prelude::*;
use futures::sync::mpsc;
use raft;
use std::collections::HashMap;
use std::net::SocketAddr;
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
            .map_err(|e| println!("Error when sending message to network: {:?}", e))
    }

    pub fn peer_ids(&self) -> &[u64] {
        &self.ids
    }

    pub fn add(&mut self, id: u64, addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
        self.ids.push(id);
        self.tx
            .clone()
            .send(Cmd::add(id, addr))
            .map(|_| ())
            .map_err(|e| println!("Error when sending message to network: {:?}", e))
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
    Add(SocketAddr),
    Raft(raft::eraftpb::Message),
}

#[derive(Debug, Clone)]
struct Cmd {
    id: u64,
    kind: Kind,
}

impl Cmd {
    fn add(id: u64, addr: &SocketAddr) -> Self {
        Cmd {
            id,
            kind: Kind::Add(addr.clone()),
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

    fn add(&mut self, id: u64, addr: SocketAddr) {
        let (tx, rx) = mpsc::channel(1024);
        self.peers.insert(id, Peer { tx, id });

        tokio::spawn(rx.for_each(move |msg| {
            let addr = addr.clone();

            // TODO: Find a way to reuse the connection
            TcpStream::connect(&addr)
                .map_err(|e| println!("Error connecting to peer: {:?}", e))
                .and_then(move |sock| {
                    let (_, sink) = sock.split();

                    let sink = FramedWrite::new(sink, Proto::<raft::eraftpb::Message>::new());
                    sink.send(msg)
                        .map(|_| ())
                        .map_err(|e| println!("Error sending message to peer: {:?}", e))
                })
        }));
    }

    fn send(&self, id: u64, msg: raft::eraftpb::Message) {
        if let Some(peer) = self.peers.get(&id) {
            tokio::spawn({
                peer.tx
                    .clone()
                    .send(msg)
                    .map(|_| ())
                    .map_err(|e| println!("Error when sending message to network: {:?}", e))
            });
        }
    }
}
