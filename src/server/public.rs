//! This module is for communications coming in from the public network. This
//! would be commands from a client.

use super::Message;
use std::net::SocketAddr;
use futures::sync::mpsc;
use futures::prelude::*;
use std::thread::{self, JoinHandle};
use tokio::net::TcpListener;
use tokio_codec::{FramedRead, FramedWrite};
use tokio::prelude::*;
use public::{self, Request, Response};
use codec::Proto;
use tokio;

type Tx = mpsc::Sender<Response>;
type Rx = mpsc::Receiver<Response>;

#[derive(Debug)]
pub struct Command {
    request: Request,
    tx: Tx,
}

impl Command {
    fn new(tx: Tx, request: Request) -> Command {
        Command { tx, request }
    }

    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn reply(self, resp: Response) {
        tokio::spawn(self.tx.send(resp).then(|_| Ok(())));
    }
}

pub fn listen(db_channel: mpsc::Sender<Message>, addr: &SocketAddr) -> Handle {
    let addr = (*addr).clone();
    let handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr).unwrap();
        let db_channel = db_channel.clone();

        let server = listener
            .incoming()
            .map_err(|e| println!("{:?}", e))
            .for_each(move |sock| {
                println!("New connection opened!");

                let (tx, rx) = mpsc::channel(1024);
                let (stream, sink) = sock.split();

                let sink = FramedWrite::new(sink, Proto::<Response>::new());
                tokio::spawn({
                    rx
                        .forward(sink.sink_map_err(handle_err))
                        .map(|_| ())
                        .map_err(handle_err)
                });


                let stream = FramedRead::new(stream, Proto::<Request>::new());
                tokio::spawn(
                    stream
                        .map_err(handle_err)
                        .map(move |request| {
                            println!("Request: {:?}", request);
                            Message::Cmd(Command::new(tx.clone(), request))
                        })
                        .forward(db_channel.clone().sink_map_err(handle_err))
                        .then(|_| Ok(()))
                );

                Ok(())
            });

        tokio::run(server);
    });

    Handle { handle }
}

fn handle_err(e: impl ::std::fmt::Debug) {
    println!("error occurred: {:?}", e);
}

pub struct Handle {
    handle: JoinHandle<()>,
}

impl Handle {
    fn join(self) {
        self.handle.join().expect("Client listener panicked");
    }
}
