extern crate clap;
extern crate kv_raft;

use kv_raft::server::{Peer, Server};
use std::net::ToSocketAddrs;

use clap::{App, Arg};

fn main() {
    let matches = App::new("Example KV")
        .arg(Arg::with_name("ID").required(true))
        .arg(
            Arg::with_name("peer-on")
                .long("peer-on")
                .short("p")
                .takes_value(true),
        )
        .arg(Arg::with_name("PEER").multiple(true).takes_value(true))
        .get_matches();

    let peers = if let Some(peers) = matches.values_of("PEER") {
        // Peers can come and go but we need to know them at the start.
        peers
            .into_iter()
            .map(|value| {
                // Simple parsing of each peer into a value
                let parts: Vec<&str> = value.splitn(2, '-').collect();

                println!("peer: {:?}", parts);

                match &parts[..] {
                    [id, addr] => Peer::new(
                        id.parse().unwrap(),
                        addr.to_socket_addrs().unwrap().next().unwrap(),
                    ),
                    _ => panic!("Invalid peers"),
                }
            })
            .collect()
    } else {
        vec![]
    };
    let id = matches.value_of("ID").unwrap_or("1").parse().unwrap();
    let peer_on = matches
        .value_of("peer-on")
        .unwrap_or("0.0.0.0:9001")
        .parse()
        .unwrap();
    Server::start(id, &peer_on, &peers).join();
}
