use std::{io};
use std::io::{ErrorKind};
use std::io::ErrorKind::Other;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::ops::Deref;
use crate::dh_layer::*;

pub struct DHSever {
    key: Key,
    established_connection: Option<SocketAddr>,
    socket: Option<Box<UdpSocket>>,
}

impl DHLayerEndpoint for DHSever {
    fn send_pkt(&self, data: &DHLayer, dst: &SocketAddr) -> Result<(), io::Error> {
        if data.content_type == DATA_TRANSMISSION {
            self.socket.as_ref().unwrap().send_to(&Self::encrypt(&data.to_bytes(), self.key), dst)?;
        } else {
            self.socket.as_ref().unwrap().send_to(&data.to_bytes(), dst)?;
        }
        Ok(())
    }

    fn recv_pkt(&self, data: &[u8], src: &SocketAddr) -> Result<(), io::Error> {
        let v = Self::decrypt(data, self.key);
        let data = v.deref();
        let x = match DHLayer::from(data) {
            None => Err(io::Error::new(ErrorKind::InvalidInput, "parse error")),
            Some(dh_layer) => {
                if *src != self.established_connection.unwrap() {
                    Err(io::Error::new(ErrorKind::Other, "unknown src"))
                } else if dh_layer.content_type == DATA_TRANSMISSION {
                    println!("recv bytes from client {:?}", dh_layer.payload.to_bytes());
                    self.send_pkt(&dh_layer, src)?;
                    Ok(())
                } else {
                    Err(io::Error::new(ErrorKind::Other, "unknown content_type"))
                }
            }
        };
        x
    }

    fn establish_connection(&mut self, data: &[u8], src: &SocketAddr) -> Result<(), io::Error> {
        self.established_connection = match DHLayer::from(data) {
            None => return Err(io::Error::new(ErrorKind::InvalidInput, "parse error")),
            Some(dh_layer) => {
                if dh_layer.content_type == HAND_SHAKE_REQUEST {
                    let [p, g, upper_a] = dh_layer.get_pg_ua().ok_or(io::Error::new(Other, "no pga found"))?;
                    let b = Self::generate_key();
                    let upper_b = Self::mod_power(g, b, p);
                    self.key = Self::mod_power(upper_a, b, p);
                    self.send_pkt(&DHLayer::new_handshake_reply(upper_b), src)?;
                    Some(src.clone())
                } else {
                    return Err(io::Error::new(ErrorKind::Other, "must HAND_SHAKE_REQUEST first"));
                }
            }
        };
        Ok(())
    }
}

impl DHSever {
    pub fn new() -> DHSever {
        DHSever { key: 0, established_connection: None, socket: None }
    }

    pub fn run<A: ToSocketAddrs>(&mut self, to_addr: A) -> Result<(), io::Error> {
        let socket = UdpSocket::bind(to_addr)?;
        self.socket = Some(Box::new(socket));
        let mut buf = [0u8; 4096];
        let (amt, src) = self.socket.as_ref().unwrap().recv_from(&mut buf)?;
        self.establish_connection(&buf[..amt], &src)?;
        loop {
            let (amt, src) = self.socket.as_ref().unwrap().recv_from(&mut buf)?;
            self.recv_pkt(&buf[..amt], &src)?;
        }
    }
}