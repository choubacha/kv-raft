use public::{Request, Response};
use raft;
use std::net::SocketAddr;
use std::thread::JoinHandle;
use futures::prelude::*;

mod db;
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
}

const PUBLIC_PORT: u16 = 9000;
const PEER_PORT: u16 = 9001;

impl Server {
    pub fn start() -> Server {
        let pub_addr = SocketAddr::new("0.0.0.0".parse().unwrap(), PUBLIC_PORT);

        let db = db::Db::new().start();
        let public = public::listen(db.channel(), &pub_addr);

        Server { db, public }
    }
}
