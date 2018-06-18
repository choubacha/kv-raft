extern crate kv_raft;

use kv_raft::client::Client;
use kv_raft::codec::Proto;
use kv_raft::server::Server;
use kv_raft::public::*;

extern crate futures;
extern crate tokio;
extern crate tokio_codec;

use futures::future::{loop_fn, Loop};
use std::thread;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_codec::{FramedRead, FramedWrite};

fn main() {
    let _ = Server::start();

    let addr = "0.0.0.0:9000".parse().unwrap();
    let client = Client::connect(&addr)
        .map_err(|e| println!("err while connecting: {:?}", e))
        .and_then(|client| {
            loop_fn((client, 0), |(client, count)| {
                client
                    .ping()
                    .map_err(|e| println!("err while getting: {:?}", e))
                    .and_then(move |(client, resp)| {
                        println!("resp: {:?}", resp);
                        if count >= 10 {
                            Ok(Loop::Break(()))
                        } else {
                            Ok(Loop::Continue((client, count + 1)))
                        }
                    })
            })
        });
    tokio::run(client);
}
