extern crate kv_raft;

use kv_raft::client::Client;
use kv_raft::codec::Proto;
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
    thread::spawn(|| {
        let addr = "127.0.0.1:6142".parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();
        let server = listener
            .incoming()
            .map_err(|e| println!("{:?}", e))
            .for_each(|sock| {
                println!("New connection opened!");

                let (stream, sink) = sock.split();

                let sink = FramedWrite::new(sink, Proto::<Response>::new());
                let stream = FramedRead::new(stream, Proto::<Request>::new());

                let (tx, rx) = futures::sync::mpsc::channel(10);

                let resp = rx.forward(sink.sink_map_err(|_| ()))
                    .map_err(|e| println!("{:?}", e))
                    .map(|_| ());

                tokio::spawn({
                    stream.map_err(|e| println!("{:?}", e)).for_each(move |r| {
                        println!("Request: {:?}", r);

                        let mut resp = Response::new();
                        let mut get = response::Get::new();
                        get.set_value(format!("Found ya: {}", r.get_get().get_key()));
                        get.set_is_found(true);

                        resp.set_get(get);

                        tokio::spawn(tx.clone().send(resp).then(|_| Ok(())));

                        Ok(())
                    })
                });
                tokio::spawn(resp);

                Ok(())
            });
        tokio::run(server);
    });

    let addr = "127.0.0.1:6142".parse().unwrap();
    let client = Client::connect(&addr)
        .map_err(|e| println!("err while connecting: {:?}", e))
        .and_then(|client| {
            loop_fn((client, 0), |(client, count)| {
                client
                    .get(&format!("get number {}", count))
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
