use std::{io};
use std::cell::Cell;
use std::io::{Error, ErrorKind};
use std::io::ErrorKind::Other;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use crate::dh_layer::*;

pub struct DHSever {
    key: Cell<Key>,
    established_connection: Cell<Option<SocketAddr>>,
    _socket: UdpSocket,
}

impl DHLayerEndpoint for DHSever {
    fn send_dh_pkt(&self, buf: &[u8], dst: &SocketAddr) -> Result<(), Error> {
        self._socket.send_to(&DHLayer::new_data_transmission(&buf.encrypted(self.key.get())), dst)?;
        Ok(())
    }

    fn recv_dh_pkt<'a>(&self, buf: &'a mut [u8]) -> Result<(DHLayer<'a>, SocketAddr), Error> {
        let (amt, src) = self._socket.recv_from(buf)?;
        match DHLayer::from(&buf[..amt]) {
            None => Err(io::Error::new(ErrorKind::InvalidInput, "parse error")),
            Some(dh_layer) => Ok((dh_layer, src))
        }
    }
}

impl DHSever {
    pub fn new<A: ToSocketAddrs>(to_addr: A) -> Result<DHSever, std::io::Error> {
        Ok(DHSever { key: Cell::new(0), established_connection: Cell::new(None), _socket: UdpSocket::bind(to_addr)? })
    }

    pub fn run(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; 4096];
        let (layer, src) = self.recv_dh_pkt(&mut buf)?;
        self.establish_connection(layer, &src)?;
        loop {
            let (dh_layer, src) = self.recv_dh_pkt(&mut buf)?;
            assert_eq!(dh_layer.content_type, DATA_TRANSMISSION);
            assert_eq!(Some(src), self.established_connection.get());
            let mut bytes = dh_layer.payload.decrypted(self.key.get());
            println!("recv bytes from client {:?}",bytes );
            let mut res = "server respond to:".as_bytes().to_vec();
            res.append(&mut bytes);
            self.send_dh_pkt(&bytes, &src)?;
        }
    }

    fn establish_connection(&self, layer: DHLayer, src: &SocketAddr) -> Result<(), io::Error> {
        let established_connection = if layer.content_type == HAND_SHAKE_REQUEST {
            let [p, g, upper_a] = layer.get_pg_ua().ok_or(io::Error::new(Other, "no pga found"))?;
            let b = Self::generate_key();
            let upper_b = Self::mod_power(g, b, p);
            self.key.set(Self::mod_power(upper_a, b, p));
            println!("recv HAND_SHAKE_REQUEST from {}, got p:{}, g:{}, A:{}", src, p, g, upper_a);
            self._socket.send_to(&DHLayer::new_handshake_reply(upper_b), src)?;
            Some(src.clone())
        } else {
            return Err(io::Error::new(ErrorKind::Other, "must HAND_SHAKE_REQUEST first"));
        };
        self.established_connection.set(established_connection);
        Ok(())
    }
}