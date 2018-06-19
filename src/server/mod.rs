use raft;
use std::net::SocketAddr;

mod db;
mod network;
mod peer;
mod proto;
mod public;

#[derive(Debug)]
pub enum Message {
    Timeout,
    Cmd(public::Command),
    Raft(raft::eraftpb::Message),
    Ping,
    Stop,
}

pub struct Server {
    db: db::Handle,
    public: public::Handle,
    peer: peer::Handle,
}

const PUBLIC_PORT: u16 = 9000;

pub struct Peer {
    id: u64,
    addr: SocketAddr,
}

impl Peer {
    pub fn new(id: u64, addr: SocketAddr) -> Peer {
        Peer { id, addr }
    }
}

impl Server {
    /// Starts a server and returns a handle to it.
    ///
    /// A server must know it's peers before it can start. Once started, this
    /// data is ignored and managed via the network. An improvement would be
    /// to all peers to be added and pass their context down with the raft
    /// message.
    pub fn start(id: u64, peer_addr: &SocketAddr, peers: &[Peer]) -> Server {
        let pub_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), PUBLIC_PORT);

        let mut network = network::start();

        // Starts up the network with peers. This helps to
        // broadcast out to peers.
        ::tokio::run(network.add(id, &peer_addr));
        for peer in peers {
            ::tokio::run(network.add(peer.id, &peer.addr));
        }

        let db = db::Db::new(id, network).start();
        let public = public::listen(db.channel(), &pub_addr);
        let peer = peer::listen(db.channel(), &peer_addr);

        Server { db, public, peer }
    }

    pub fn join(self) {
        self.db.join();
        self.public.join();
        self.peer.join();
    }
}
