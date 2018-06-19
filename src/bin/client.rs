extern crate clap;
extern crate futures;
extern crate kv_raft;
extern crate protobuf;
extern crate raft;
extern crate tokio;
extern crate tokio_codec;

use clap::{App, Arg, SubCommand};
use kv_raft::client::Client;
use tokio::prelude::*;

fn main() {
    let matches = App::new("Client")
        .arg(Arg::with_name("host").short("h").takes_value(true))
        .subcommand(SubCommand::with_name("get").arg(Arg::with_name("KEY").takes_value(true)))
        .subcommand(SubCommand::with_name("delete").arg(Arg::with_name("KEY").takes_value(true)))
        .subcommand(
            SubCommand::with_name("set")
                .arg(Arg::with_name("KEY").takes_value(true))
                .arg(Arg::with_name("VALUE").takes_value(true)),
        )
        .subcommand(
            SubCommand::with_name("add_node")
                .arg(Arg::with_name("ID").takes_value(true))
                .arg(Arg::with_name("ADDR").takes_value(true))
                .arg(
                    Arg::with_name("learner")
                        .long("learner")
                        .help("Add as a learner node"),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove_node").arg(Arg::with_name("ID").takes_value(true)),
        )
        .subcommand(SubCommand::with_name("scan"))
        .subcommand(SubCommand::with_name("ping"))
        .subcommand(SubCommand::with_name("bench"))
        .get_matches();

    let addr = matches
        .value_of("host")
        .unwrap_or("0.0.0.0:9000")
        .parse()
        .unwrap();

    let task = Client::connect(&addr)
        .map_err(|e| println!("err while connecting: {:?}", e))
        .and_then(move |client| {
            match matches.subcommand() {
                ("get", Some(sub)) => {
                    let key = sub.value_of("KEY").unwrap();
                    ::tokio::spawn(
                        client
                            .get(&key)
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(_, resp)| {
                                let resp = resp.expect("Response missing");

                                if resp.get_get().get_is_found() {
                                    println!("{}", resp.get_get().get_value());
                                } else {
                                    println!("Key not found");
                                    ::std::process::exit(1);
                                }
                                Ok(())
                            }),
                    );
                }
                ("set", Some(sub)) => {
                    let key = sub.value_of("KEY").unwrap();
                    let value = sub.value_of("VALUE").unwrap();
                    ::tokio::spawn(
                        client
                            .set(&key, &value)
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(_, resp)| {
                                let resp = resp.expect("Response missing");
                                if !resp.get_success() {
                                    println!("Value not set");
                                    ::std::process::exit(1);
                                }
                                Ok(())
                            }),
                    );
                }
                ("add_node", Some(sub)) => {
                    let id = sub.value_of("ID").unwrap().parse().unwrap();
                    let addr = sub.value_of("ADDR").unwrap();
                    let is_learner = sub.is_present("learner");
                    ::tokio::spawn(
                        client
                            .add_node(id, addr.to_string(), is_learner)
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(_, resp)| {
                                let resp = resp.expect("Response missing");
                                if !resp.get_success() {
                                    println!("Node failed to add");
                                    ::std::process::exit(1);
                                }
                                Ok(())
                            }),
                    );
                }
                ("remove_node", Some(sub)) => {
                    let id = sub.value_of("ID").unwrap().parse().unwrap();
                    ::tokio::spawn(
                        client
                            .remove_node(id)
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(_, resp)| {
                                let resp = resp.expect("Response missing");
                                if !resp.get_success() {
                                    println!("Node failed to remove");
                                    ::std::process::exit(1);
                                }
                                Ok(())
                            }),
                    );
                }
                ("delete", Some(sub)) => {
                    let key = sub.value_of("KEY").unwrap();
                    ::tokio::spawn(
                        client
                            .delete(&key)
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(_, resp)| {
                                let resp = resp.expect("Response missing");

                                if resp.get_delete().get_is_found() {
                                    println!("{}", resp.get_delete().get_value());
                                } else {
                                    println!("Key not found");
                                    ::std::process::exit(1);
                                }
                                Ok(())
                            }),
                    );
                }
                ("scan", Some(_)) => {
                    ::tokio::spawn(
                        client
                            .scan()
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |(_, resp)| {
                                let resp = resp.expect("Response missing");

                                for key in resp.get_scan().get_keys() {
                                    println!("{}", key);
                                }
                                Ok(())
                            }),
                    );
                }
                ("ping", Some(_)) => {
                    ::tokio::spawn(
                        client
                            .ping()
                            .map_err(|e| println!("err while setting: {:?}", e))
                            .and_then(move |_| {
                                println!("pong");
                                Ok(())
                            }),
                    );
                }
                _ => panic!("No option chosen"),
            }
            Ok(())
        });

    ::tokio::run(task);
}
