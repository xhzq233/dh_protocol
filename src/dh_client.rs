use std::io;
use std::io::{Error, ErrorKind};
use std::io::ErrorKind::{InvalidInput, Other};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use crate::dh_layer::*;

pub struct DHClient {
    pub key: Key,
    p: Key,
    g: Key,
    a: Key,
    established_connection: Option<SocketAddr>,
    socket: UdpSocket,
}

impl DHClient {
    pub fn new<A: ToSocketAddrs>(to_addr: A) -> Result<DHClient, std::io::Error> {
        let p = Self::generate_key(64);
        let g = Self::get_primitive_root(p).ok_or(std::io::Error::new(Other, "cant find primitive_root"))?;
        let a = Self::generate_key(128);
        Ok(DHClient { key: 0, p, g, a, established_connection: None, socket: UdpSocket::bind(to_addr)? })
    }

    pub fn establish_connection<A: ToSocketAddrs>(&mut self, to_addr: A) -> Result<(), std::io::Error> {
        let dst = to_addr.to_socket_addrs()?.next().ok_or(std::io::Error::new(Other, "parse addr err"))?;
        // establish connection
        let upper_a = Self::mod_power(self.g, self.a, self.p);
        self.socket.send_to(&DHLayer::new_handshake_request(self.p, self.g, upper_a), dst)?;
        let mut buf = [0; 256];
        let (dh_layer, src_addr) = self.recv_dh_pkt(&mut buf)?;
        if src_addr != dst {
            Err(std::io::Error::new(Other, "src dont match"))
        } else {
            if dh_layer.content_type == HAND_SHAKE_REPLY {
                let upper_b = dh_layer.get_ub().ok_or(std::io::Error::new(InvalidInput, "no b"))?;
                self.key = Self::mod_power(upper_b, self.a, self.p);
                println!("handshake success! key {}",self.key);
                self.established_connection = Some(src_addr);
                Ok(())
            } else {
                Err(std::io::Error::new(Other, "bad reply"))
            }
        }
    }

    pub fn send_to(&self, data: &[u8]) -> Result<(), std::io::Error> {
        if self.established_connection == None {
            return Err(std::io::Error::new(Other, "no established_connection yet"));
        }
        self.send_dh_pkt(data, &self.established_connection.unwrap())?;
        Ok(())
    }
}

impl DHLayerEndpoint for DHClient {
    fn send_dh_pkt(&self, buf: &[u8], dst: &SocketAddr) -> Result<(), Error> {
        self.socket.send_to(&DHLayer::new_data_transmission(&buf.encrypted(self.key)), dst)?;
        Ok(())
    }
    fn recv_dh_pkt<'a>(&self, buf: &'a mut [u8]) -> Result<(DHLayer<'a>, SocketAddr), Error> {
        let (amt, src) = self.socket.recv_from(buf)?;
        match DHLayer::from(&buf[..amt]) {
            None => Err(io::Error::new(ErrorKind::InvalidInput, "parse error")),
            Some(dh_layer) => Ok((dh_layer, src))
        }
    }
}