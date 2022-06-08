use std::io;
use std::io::ErrorKind;
use std::io::ErrorKind::{InvalidInput, Other};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::{from_utf8};
use crate::dh_layer::*;

pub struct DHClient {
    key: Key,
    p: Key,
    g: Key,
    a: Key,
    established_connection: Option<SocketAddr>,
    socket: UdpSocket,
}

impl DHClient {
    pub fn new<A: ToSocketAddrs>(to_addr: A) -> Result<DHClient, std::io::Error> {
        let p = Self::generate_key();
        let g = Self::get_primitive_root(p).ok_or(std::io::Error::new(Other, "cant find primitive_root"))?;
        let a = Self::generate_key();
        Ok(DHClient { key: 0, p, g, a, established_connection:None, socket: UdpSocket::bind(to_addr)? })
    }

    pub fn on_recv(&self) -> Result<(), std::io::Error> {
        let mut buf = [0u8; 4096];
        loop {
            let (amt, src) = self.socket.recv_from(&mut buf)?;
            self.recv_pkt(&buf[..amt], &src)?;
        }
    }

    pub fn establish_connection<A: ToSocketAddrs>(&mut self, to_addr: A) -> Result<(), std::io::Error>  {
        let dst = to_addr.to_socket_addrs()?.next().ok_or(std::io::Error::new(Other, "parse addr err"))?;
        // establish connection
        let upper_a = Self::mod_power(self.g, self.a, self.p);
        self.send_pkt(&DHLayer::new_handshake_request(self.p, self.g, upper_a), &dst)?;
        let mut buf = [0; 200];
        let (number_of_bytes, src_addr) = self.socket.recv_from(&mut buf)?;
        if src_addr != dst {
            Err(std::io::Error::new(Other, "src dont match"))
        } else {
            if let Some(dh_layer) = DHLayer::from(&buf[..number_of_bytes]) {
                if dh_layer.content_type == HAND_SHAKE_REPLY {
                    let upper_b = dh_layer.get_ub().ok_or(std::io::Error::new(InvalidInput, ""))?;
                    self.key = Self::mod_power(upper_b, self.a, self.p);
                    self.established_connection = Some(src_addr);
                    Ok(())
                } else {
                    Err(std::io::Error::new(Other, "bad reply"))
                }
            } else {
                Err(std::io::Error::new(Other, "cant parse"))
            }
        }
    }

    pub fn send_to<T: ToBytes>(&self, data: T) -> Result<(), std::io::Error> {
        if self.established_connection == None {
            return Err(std::io::Error::new(Other, "src dont match"));
        }
        self.send_pkt(&DHLayer::new_data_transmission(data.to_bytes()), &self.established_connection.unwrap())?;
        Ok(())
    }
}

impl DHLayerEndpoint for DHClient {
    fn send_pkt(&self, data: &DHLayer, dst: &SocketAddr) -> Result<(), std::io::Error> {
        if data.content_type == DATA_TRANSMISSION {
            self.socket.send_to(&Self::encrypt(&data.to_bytes(), self.key), dst)?;
        } else {
            self.socket.send_to(&data.to_bytes(), dst)?;
        }
        Ok(())
    }

    fn recv_pkt(&self, data: &[u8], src: &SocketAddr) -> Result<(), std::io::Error> {
        let v = Self::decrypt(data, self.key);
        let data = &*v;
        let x = match DHLayer::from(data) {
            None => Err(io::Error::new(ErrorKind::InvalidInput, "parse layer error")),
            Some(dh_layer) => {
                match from_utf8(dh_layer.payload.to_bytes()) {
                    Ok(str) => {
                        println!("recv utf8: '{}', from {}", str, src);
                        Ok(())
                    }
                    Err(e) => {
                        println!("recv bytes {:?}, from {}", data, src);
                        Err(std::io::Error::new(Other, e.to_string()))
                    }
                }
            }
        };
        x
    }
}