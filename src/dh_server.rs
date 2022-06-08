use std::{io};
use std::cell::Cell;
use std::io::{ErrorKind};
use std::io::ErrorKind::Other;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::ops::Deref;
use crate::dh_layer::*;

pub struct DHSever {
    key: Cell<Key>,
    established_connection: Cell<Option<SocketAddr>>,
    _socket: UdpSocket,
}

impl DHLayerEndpoint for DHSever {
    fn send_pkt(&self, data: &DHLayer, dst: &SocketAddr) -> Result<(), io::Error> {
        if data.content_type == DATA_TRANSMISSION {
            self._socket.send_to(&Self::encrypt(&data.to_bytes(), self.key.get()), dst)?;
        } else {
            self._socket.send_to(&data.to_bytes(), dst)?;
        }
        Ok(())
    }

    fn recv_pkt(&self, data: &[u8], src: &SocketAddr) -> Result<(), io::Error> {
        let v = Self::decrypt(data, self.key.get());
        let data = v.deref();
        let x = match DHLayer::from(data) {
            None => Err(io::Error::new(ErrorKind::InvalidInput, "parse error")),
            Some(dh_layer) => {
                if Some(*src) != self.established_connection.get() {
                    Err(io::Error::new(ErrorKind::Other, "unknown src"))
                } else if dh_layer.content_type == DATA_TRANSMISSION {
                    println!("recv bytes from client {:?}", dh_layer.payload.to_bytes());
                    self.send_pkt(&DHLayer::new_data_transmission("hello client, this is server".to_bytes()), src)?;
                    Ok(())
                } else {
                    Err(io::Error::new(ErrorKind::Other, "unknown content_type"))
                }
            }
        };
        x
    }
}

impl DHSever {
    pub fn new<A: ToSocketAddrs>(to_addr: A) ->  Result<DHSever, std::io::Error> {
        Ok(DHSever { key: Cell::new(0), established_connection: Cell::new(None), _socket: UdpSocket::bind(to_addr)? })
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 4096];
        let (amt, src) = self._socket.recv_from(&mut buf)?;
        self.establish_connection(&buf[..amt], &src)?;
        loop {
            let (amt, src) = self._socket.recv_from(&mut buf)?;
            self.recv_pkt(&buf[..amt], &src)?;
        }
    }

    fn establish_connection(&self, data: &[u8], src: &SocketAddr) -> Result<(), io::Error> {
        let established_connection = match DHLayer::from(data) {
            None => return Err(io::Error::new(ErrorKind::InvalidInput, "parse error")),
            Some(dh_layer) => {
                if dh_layer.content_type == HAND_SHAKE_REQUEST {
                    let [p, g, upper_a] = dh_layer.get_pg_ua().ok_or(io::Error::new(Other, "no pga found"))?;
                    let b = Self::generate_key();
                    let upper_b = Self::mod_power(g, b, p);
                    self.key.set(Self::mod_power(upper_a, b, p));
                    self.send_pkt(&DHLayer::new_handshake_reply(upper_b), src)?;
                    Some(src.clone())
                } else {
                    return Err(io::Error::new(ErrorKind::Other, "must HAND_SHAKE_REQUEST first"));
                }
            }
        };
        self.established_connection.set(established_connection);
        Ok(())
    }
}