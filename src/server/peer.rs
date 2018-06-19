//! The peer module starts a listener for raft messages only.
//! These message are wrapped into a server message and forwarded
//! to the db channel.

use super::Message;
use codec::Proto;
use futures::prelude::*;
use futures::sync::mpsc;
use raft;
use std::net::SocketAddr;
use std::thread::{self, JoinHandle};
use tokio;
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio_codec::FramedRead;

pub fn listen(db_channel: mpsc::Sender<Message>, addr: &SocketAddr) -> Handle {
    let addr = (*addr).clone();
    let handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr).unwrap();
        let db_channel = db_channel.clone();

        let server = listener
            .incoming()
            .map_err(handle_err)
            .for_each(move |sock| {
                println!("New peer connected!");

                let (stream, _) = sock.split();

                let stream = FramedRead::new(stream, Proto::<raft::eraftpb::Message>::new());
                tokio::spawn(
                    stream
                        .map_err(handle_err)
                        .map(move |msg| Message::Raft(msg))
                        .forward(db_channel.clone().sink_map_err(handle_err))
                        .then(|_| Ok(())),
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
    pub fn join(self) {
        self.handle.join().expect("Client listener panicked");
    }
}
