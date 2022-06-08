use std::{io, u128};
use std::net::{SocketAddr};
use num_prime::RandPrime;
use rand::{thread_rng};

pub type Key = u128;

pub const HAND_SHAKE_REQUEST: u8 = 1;
pub const HAND_SHAKE_REPLY: u8 = 2;
pub const DATA_TRANSMISSION: u8 = 3;
const DH_IDENTIFIER: [u8; 2] = ['D' as u8, 'H' as u8];

pub trait DHLayerEndpoint {
    //将buf添加到udp层上发送
    fn send_dh_pkt(&self, buf: &[u8], dst: &SocketAddr) -> Result<(), io::Error>;

    //接受来自upd的payload, 转成dh layer后 返回dh layer的payload的大小
    fn recv_dh_pkt<'a>(&self, buf: &'a mut [u8]) -> Result<(DHLayer<'a>, SocketAddr), io::Error>;

    //建立连接
    // fn establish_connection(&self, data: &[u8], addr: &SocketAddr) -> Result<(), io::Error>;

    // return a prime
    fn generate_key() -> Key {
        let mut rng = thread_rng();
        rng.gen_prime(127, None)
    }

    fn get_primitive_root(prime: Key) -> Option<Key> {
        let k = (prime - 1) >> 1;
        println!("computing primitive_root of {}", prime);
        for i in (2..prime / 2).rev() {//从高处开始找，找大数
            if Self::mod_power(i, k, prime) != 1 {
                println!("find! {}", i);
                return Some(i);
            }
        }
        None
    }

    //g^power mod p
    fn mod_power(g: Key, power: Key, p: Key) -> Key {
        let mut res: Key = 1;
        let mut g = g % p;
        let mut power = power;
        while power > 0 {
            if (power & 0x01) == 0x01 {
                res = (res * g) % p;
            }
            power = power >> 1;
            g = (g * g) % p;
        }
        res
    }
}

pub struct DHLayer<'a> {
    // constant value: ['D','H']
    pub dh_identifier: [u8; 2],
    // 1 or 2 or 3
    pub content_type: u8,
    // 1 => 3*16, length of p and g and upper_a
    // 2 => 1*16, length of upper_b
    // 3 => length of data(payload)
    pub length: u32,
    // p + g + upper_a when type is 1,
    // upper_b when type is 2,
    // data when type is 3
    pub payload: &'a [u8],
}

pub trait Crypt{
    // 加密data todo()
    fn encrypted(&self,key: Key) -> Vec<u8>;
    // 解密data
    fn decrypted(&self,key: Key) -> Vec<u8>;
}

impl Crypt for &[u8] {
    fn encrypted(&self, key: Key) -> Vec<u8> {
        let bs: [u8; 16] = key.to_le_bytes();
        let mut v = vec![0; self.len()];
        for (i, value) in self.iter().enumerate() {
            v[i] = value ^ bs[i % 16];
        }
        v
    }

    fn decrypted(&self, key: Key) -> Vec<u8> {
        self.encrypted(key)
    }
}

impl<'a> DHLayer<'a> {
    pub fn new_handshake_request(p: Key, g: Key, upper_a: Key) -> Vec<u8> {
        let mut v = p.to_le_bytes().to_vec();
        v.append(&mut g.to_le_bytes().to_vec());
        v.append(&mut upper_a.to_le_bytes().to_vec());
        DHLayer {
            dh_identifier: DH_IDENTIFIER,
            content_type: HAND_SHAKE_REQUEST,
            length: 16 * 3,
            payload: v.as_slice(),
        }.to_bytes()
    }

    pub fn new_handshake_reply(upper_b: Key) -> Vec<u8> {
        DHLayer {
            dh_identifier: DH_IDENTIFIER,
            content_type: HAND_SHAKE_REPLY,
            length: 16,
            payload: &upper_b.to_le_bytes(),
        }.to_bytes()
    }

    pub fn new_data_transmission(data: &'a [u8]) -> Vec<u8> {
        DHLayer {
            dh_identifier: DH_IDENTIFIER,
            content_type: DATA_TRANSMISSION,
            length: data.len() as u32,
            payload: data,
        }.to_bytes()
    }

    pub fn from(udp_payload: &[u8]) -> Option<DHLayer> {
        if matches!(udp_payload[0..2].try_into().ok(),Some(DH_IDENTIFIER)) {
            let content_type = udp_payload[2];
            if content_type > DATA_TRANSMISSION || content_type < HAND_SHAKE_REQUEST {
                None
            } else {
                let length = u32::from_le_bytes(udp_payload[2..6].try_into().ok()?);
                Some(DHLayer {
                    dh_identifier: DH_IDENTIFIER,
                    content_type,
                    length,
                    payload: &udp_payload[7..],
                })
            }
        } else {
            None
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = vec![0u8; 2 + 1 + 4];
        res[0] = DH_IDENTIFIER[0];
        res[1] = DH_IDENTIFIER[1];
        res[2] = self.content_type;
        res[6] = (self.length >> 24) as u8;
        res[5] = (self.length >> 16) as u8;
        res[4] = (self.length >> 8) as u8;
        res[3] = self.length as u8;
        res.append(&mut self.payload.to_vec());
        res
    }

    pub fn get_pg_ua(&self) -> Option<[Key; 3]> {
        if self.content_type != HAND_SHAKE_REQUEST {
            None
        } else {
            let bytes = self.payload;
            let p = u128::from_le_bytes(bytes[..16].try_into().ok()?);
            let g = u128::from_le_bytes(bytes[16..32].try_into().ok()?);
            let upper_a = u128::from_le_bytes(bytes[32..48].try_into().ok()?);
            Some([p, g, upper_a])
        }
    }

    pub fn get_ub(&self) -> Option<Key> {
        if self.content_type != HAND_SHAKE_REPLY {
            None
        } else {
            let b = u128::from_le_bytes(self.payload[..16].try_into().ok()?);
            Some(b)
        }
    }
}