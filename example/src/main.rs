extern crate kv_raft;

use kv_raft::codec::Proto;
use kv_raft::public::*;

extern crate futures;
extern crate tokio;
extern crate tokio_codec;

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio_codec::{FramedRead, FramedWrite, Decoder};
use std::{thread, time::Duration};
use std::sync::RwLock;

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

                let resp = rx
                    .forward(sink.sink_map_err(|_| ()))
                    .map_err(|e| println!("{:?}", e))
                    .map(|_| ());

                tokio::spawn({
                    stream
                        .map_err(|e| println!("{:?}", e))
                        .for_each(move |r| {
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
    let client = TcpStream::connect(&addr)
        .map_err(|e| println!("{:?}", e))
        .map(move |sock| {

            let (stream, sink) = sock.split();

            let sink = FramedWrite::new(sink, Proto::<Request>::new());
            let stream = FramedRead::new(stream, Proto::<Response>::new());

            let num_requests = 10;

            // Send 10 requests
            tokio::spawn(
                futures::stream::iter_ok(0..num_requests)
                    .map(|i| {
                        let mut request = Request::new();
                        let mut get = request::Get::new();
                        get.set_key(format!("msg # {}", i));
                        request.set_get(get);
                        request
                    })
                    .forward(sink.sink_map_err(|_| ()))
                    .and_then(|(_, mut sink)| {
                        sink.close();
                        Ok(())
                    })
            );

            // Take 10 responses
            tokio::spawn(
                stream
                    .map_err(|e| println!("{:?}", e))
                    .take(num_requests)
                    .for_each(|r| {
                        println!("Response: {:?}", r);
                        Ok(())
                    })
            );
        });

    tokio::run(client);
}
