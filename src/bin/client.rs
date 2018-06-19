extern crate clap;
extern crate futures;
extern crate kv_raft;
extern crate protobuf;
extern crate raft;
extern crate tokio;
extern crate tokio_codec;

use clap::{App, Arg};
use futures::future::{loop_fn, Loop};
use kv_raft::client::Client;
use tokio::prelude::*;

fn main() {
    let matches = App::new("Client")
        .arg(Arg::with_name("host").short("h").takes_value(true))
        .get_matches();

    let addr = matches
        .value_of("host")
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
    ::tokio::run(client);
}
