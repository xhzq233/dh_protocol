use std::io::Error;
use std::str::{from_utf8};
use std::sync::Arc;
use std::thread;
use crate::dh_client::DHClient;
use crate::dh_server::DHSever;

mod dh_client;
mod dh_layer;
mod dh_server;

use clap::{Arg, App};
use crate::dh_layer::{Crypt, DHLayerEndpoint};

unsafe impl Send for DHClient {}

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
        .arg(Arg::with_name("ip")
            .help("host ip address")
            .short("i")
            .long("ip")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("dst")
            .help("the destination ip addr where the client sends the message")
            .short("d")
            .long("dst_ip")
            .takes_value(true)
            .required(false))
        .get_matches();

    let use_client = matches.is_present("client");
    let use_server = matches.is_present("server");

    let ip = matches.value_of("ip").unwrap();
    if use_client ^ use_server {
        if use_server {
            let server = DHSever::new((ip, 23334))?;
            println!("server starting");
            server.run()?;
        } else {
            let dst = matches.value_of("dst").expect("need --dst: the destination ip addr where the client sends the message");
            let mut client = DHClient::new((ip, 23333))?;
            client.establish_connection((dst, 23334))?;
            let client = Arc::new(client);
            let client_clone = client.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    let (layer, src) = client_clone.recv_dh_pkt(&mut buf).expect("recv_dh_pkt err");
                    match from_utf8(&layer.payload.decrypted(client_clone.key)) {
                        Ok(str) => { println!("recv utf8 '{}' from {}", str, src); }
                        Err(e) => { println!("error:{}", e); }
                    }
                }
            });
            let mut line = String::new();
            loop {
                let _ = std::io::stdin().read_line(&mut line).expect("read_line failed!!");
                client.send_to(line.as_bytes())?;
                line.clear();
            }
        }
    } else {
        eprintln!("args error, use -h for more info");
    }
    Ok(())
}