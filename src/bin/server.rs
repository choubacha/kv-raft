extern crate clap;
extern crate kv_raft;

use kv_raft::server::Server;

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
        .arg(
            Arg::with_name("data-file")
                .long("data-file")
                .short("f")
                .takes_value(true),
        )
        .get_matches();

    let id = matches.value_of("ID").unwrap_or("1").parse().unwrap();
    let peer_on = matches
        .value_of("peer-on")
        .unwrap_or("0.0.0.0:9001")
        .to_string();

    let file = matches.value_of("data-file").unwrap_or("/tmp/data");
    Server::start(id, &file, peer_on).join();
}
