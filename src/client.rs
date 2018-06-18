use codec::{self, Proto};
use futures::Future;
use public::{self, Request, Response};
use std::net::SocketAddr;
use tokio::io::{AsyncRead, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::{FramedRead, FramedWrite};

/// The client with-which to access the key-value database. The client
/// is a consuming struct that provides access back to itself in the future.
/// Unless there's an error, then you should reconnect.
pub struct Client {
    sink: FramedWrite<WriteHalf<TcpStream>, Proto<Request>>,
    stream: FramedRead<ReadHalf<TcpStream>, Proto<Response>>,
}

type ClientResponse = (Client, Option<Response>);

impl Client {
    pub fn connect(addr: &SocketAddr) -> impl Future<Item = Client, Error = ::std::io::Error> {
        TcpStream::connect(&addr).map(move |sock| {
            let (stream, sink) = sock.split();

            let sink = FramedWrite::new(sink, Proto::<Request>::new());
            let stream = FramedRead::new(stream, Proto::<Response>::new());

            Client { sink, stream }
        })
    }

    pub fn get(self, key: &str) -> impl Future<Item = ClientResponse, Error = codec::Error> {
        self.send(public::get_request(&key))
    }

    pub fn ping(self) -> impl Future<Item = ClientResponse, Error = codec::Error> {
        self.send(public::ping_request())
    }

    pub fn set(
        self,
        key: &str,
        value: &str,
    ) -> impl Future<Item = ClientResponse, Error = codec::Error> {
        self.send(public::set_request(&key, &value))
    }

    fn send(self, request: Request) -> impl Future<Item = ClientResponse, Error = codec::Error> {
        let sink = self.sink;
        let stream = self.stream;

        sink.send(request).and_then(|sink| {
            stream
                .into_future()
                .map(|(item, stream)| (Client { sink, stream }, item))
                .map_err(|(e, _)| e)
        })
    }
}
