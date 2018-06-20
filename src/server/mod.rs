use raft;
use std::net::SocketAddr;

mod db;
mod network;
mod peer;
mod proto;
mod public;
mod storage;

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

impl Server {
    /// Starts a server and returns a handle to it.
    ///
    /// A server must know it's peers before it can start. Once started, this
    /// data is ignored and managed via the network. An improvement would be
    /// to all peers to be added and pass their context down with the raft
    /// message.
    pub fn start(id: u64, file: &str, peer_addr: String) -> Server {
        let pub_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), 9000);

        let mut network = network::start();

        // Always add self to the network
        ::tokio::run(network.add(id, peer_addr.clone()));

        let db = db::Db::new(id, &file, network).start();
        let public = public::listen(db.channel(), &pub_addr);
        let peer = peer::listen(db.channel(), &peer_addr.parse().unwrap());

        Server { db, public, peer }
    }

    pub fn join(self) {
        self.db.join();
        self.peer.join();
        self.public.join();
    }
}
