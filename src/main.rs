use std::io::Error;
use crate::dh_client::DHClient;
// use crate::dh_layer::DHLayerEndpoint;
use crate::dh_server::DHSever;

mod dh_client;
mod dh_layer;
mod dh_server;

use clap::{Arg, App};

fn main() -> Result<(), Error> {
    let matches = App::new("'DH' protocol design")
        .author("xhzq")
        .arg(Arg::with_name("client")
            .help("run as client")
            .short("c")
            .required(false))
        .arg(Arg::with_name("server")
            .help("run as server")
            .short("s")
            .required(false))
        .get_matches_from(vec![
            "dh_protocol", "-c"
        ]);

    let use_client = matches.is_present("client");
    let use_server = matches.is_present("server");
    if use_client ^ use_server {
        if use_server {
            let mut server = DHSever::new();
            server.run(("127.0.0.1", 23334))?;
        } else {
            let mut client = DHClient::new(("127.0.0.1", 23333))?;
            client.send_to("xhzq", ("127.0.0.1", 23334))?;
            client.on_recv()?;
        }
    } else {
        eprintln!("args error, use -h for more info");
    }
    Ok(())
}