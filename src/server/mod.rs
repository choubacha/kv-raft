use futures::prelude::*;
use public::{Request, Response};
use raft;
use std::net::SocketAddr;
use std::thread::JoinHandle;

mod db;
mod peer;
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
const PEER_PORT: u16 = 9001;

impl Server {
    pub fn start() -> Server {
        let pub_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), PUBLIC_PORT);
        let peer_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), PEER_PORT);

        let db = db::Db::new().start();
        let public = public::listen(db.channel(), &pub_addr);
        let peer = peer::listen(db.channel(), &peer_addr);

        Server { db, public, peer }
    }
}
