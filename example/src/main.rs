extern crate clap;
extern crate kv_raft;

use kv_raft::client::Client;
use kv_raft::codec::Proto;
use kv_raft::public::*;
use kv_raft::server::Server;

extern crate futures;
extern crate tokio;
extern crate tokio_codec;

use futures::future::{loop_fn, Loop};
use std::thread;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_codec::{FramedRead, FramedWrite};
use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("Example KV")
        .subcommand(
            SubCommand::with_name("client")
                .arg(Arg::with_name("host").short("h").takes_value(true)),
        )
        .subcommand(
            SubCommand::with_name("server")
                .arg(Arg::with_name("ID").required(true))
                .arg(Arg::with_name("PEER").multiple(true).takes_value(true)),
        )
        .get_matches();

    match matches.subcommand() {
        ("client", Some(sub)) => {
            let addr = sub.value_of("host")
                .unwrap_or("0.0.0.0:9000")
                .parse()
                .unwrap();
            let client = Client::connect(&addr)
                .map_err(|e| println!("err while connecting: {:?}", e))
                .and_then(|client| {
                    loop_fn((client, 0), |(client, count)| {
                        let key = format!("key-{}", count);
                        client
                            .set(&key, &format!("value: {}", count))
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(client, resp)| {
                                println!("set resp: {:?}", resp);

                                client
                                    .get(&key)
                                    .map_err(|e| println!("err while getting: {:?}", e))
                                    .and_then(move |(client, resp)| {
                                        println!("get resp: {:?}", resp);
                                        if count >= 100 {
                                            Ok(Loop::Break(()))
                                        } else {
                                            Ok(Loop::Continue((client, count + 1)))
                                        }
                                    })
                            })
                    })
                });
            tokio::run(client);
        }
        ("server", Some(sub)) => {
            Server::start(1, &vec![]).join();
        }
        _ => println!("No valid command selected"),
    }
}
