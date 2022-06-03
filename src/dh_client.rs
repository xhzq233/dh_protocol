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
        Ok(DHClient { key: 0, p, g, a, established_connection: None, socket: UdpSocket::bind(to_addr)? })
    }

    pub fn on_recv(&self) -> Result<(), std::io::Error> {
        let mut buf = [0u8; 4096];
        loop {
            let (amt, src) = self.socket.recv_from(&mut buf)?;
            self.recv_pkt(&buf[..amt], &src)?;
        }
    }

    pub fn send_to<T: ToBytes, A: ToSocketAddrs>(&mut self, data: T, to_addr: A) -> Result<(), std::io::Error> {
        let dst = to_addr.to_socket_addrs()?.next().ok_or(std::io::Error::new(Other, "parse addr err"))?;
        if self.established_connection == None {
            // establish connection
            let upper_a = Self::mod_power(self.g, self.a, self.p);
            self.send_pkt(&DHLayer::new_handshake_request(self.p, self.g, upper_a), &dst)?;
            let mut buf = [0; 200];
            let (number_of_bytes, src_addr) = self.socket.recv_from(&mut buf)?;
            if src_addr != dst {
                return Err(std::io::Error::new(Other, "src dont match"));
            } else {
                self.establish_connection(&buf[..number_of_bytes], &src_addr)?;
            }
        }
        self.send_pkt(&DHLayer::new_data_transmission(data.to_bytes()), &dst)?;
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
        match from_utf8(data) {
            Ok(str) => {
                println!("recv utf8 {} from {}", str, src);
                Ok(())
            }
            Err(e) => {
                println!("recv bytes {:?} from {}", data, src);
                Err(std::io::Error::new(Other, e.to_string()))
            }
        }
    }

    fn establish_connection(&mut self, data: &[u8], src: &SocketAddr) -> Result<(), std::io::Error> {
        if let Some(dh_layer) = DHLayer::from(data) {
            if dh_layer.content_type == HAND_SHAKE_REPLY {
                let upper_b = dh_layer.get_ub().ok_or(std::io::Error::new(InvalidInput, ""))?;
                self.key = Self::mod_power(upper_b, self.a, self.p);
                self.established_connection = Some(src.clone());
                Ok(())
            } else {
                Err(std::io::Error::new(Other, "bad reply"))
            }
        } else {
            Err(std::io::Error::new(Other, "cant parse"))
        }
    }
}