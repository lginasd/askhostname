// Multicast DNS
// https://datatracker.ietf.org/doc/html/rfc6762

use std::net::{SocketAddr, IpAddr};
use socket2::{Socket, Domain, Type, Protocol};
use crate::QueryError;
use crate::net::DnsHeader;
use crate::net::query;

#[repr(C)]
pub struct MdnsQuery {
    header: DnsHeader,

    qname: Vec<u8>, // is dynamic size
    qtype: u16,
    qclass: u16 // first bit is UNICAST-RESPONSE flag for QU (query unicast), which desires
                // unicast respose back to the host
}
// Unicast direct reverse DNS lookup query with unicast response directly to the host
impl MdnsQuery {
    pub const PORT: u16 = 5353;

    fn new(ip: IpAddr) -> Self {
        let mut question = vec![];

        match ip {
            IpAddr::V4(a) => {
                let octets: Vec<String> = a.octets().into_iter().map(|x| x.to_string()).collect();
                for octet in octets.iter().rev() {
                    question.push(octet.len() as u8);
                    octet.chars().for_each(|o| { question.push(o as u8); });
                }
            },
            IpAddr::V6(_a) => {
                todo!()
            }
        };

        question.push(7);
        "in-addr".chars().for_each(|c| question.push(c as u8));
        question.push(4);
        "arpa".chars().for_each(|c| question.push(c as u8));
        question.push(0);

        MdnsQuery {
            header: DnsHeader::new_mdns(),

            qname: question,
            qtype: 0x000cu16.to_be(),
            qclass: 0x0001u16.to_be()
        }
    }
    fn header_as_slice(&self) -> &[u8; DnsHeader::SIZE] {
        unsafe {
            &*(self as *const Self as *const [u8; DnsHeader::SIZE])
        }
    }

    fn message(&self) -> Vec<u8> {
        let mut tmp_vec: Vec<u8> = vec![];
        self.header_as_slice().map(|b| tmp_vec.push(b));
        self.qname.iter().for_each(|b| tmp_vec.push(*b));

        tmp_vec.push(self.qtype as u8);
        tmp_vec.push((self.qtype >> 8) as u8);

        tmp_vec.push(self.qclass as u8);
        tmp_vec.push((self.qclass >> 8) as u8);

        tmp_vec
    }

    pub fn send(addr: &str) -> Result<String, QueryError> {

        // let ip: SocketAddr = format!("{}:0", addr).parse().unwrap();
        // let ip = ip.ip();
        let ip = addr.parse().expect("Ip parse failed. MDNS send");
        let request = Self::new(ip).message();

        let buff = query(addr, Self::PORT, &request)?;

        // response contain request + time to live [0u8; 4] + answer
        // the next two bytes correspond to the answer size
        let (_, response) = buff.split_at(request.len() + 10);
        let name_size: u16 = ((response[0] as u16) << 8) | response[1] as u16;

        let mut name = String::new();

        // println!("Response: {:x?}", response);

        // let word_size = response[2];
        response[3..].iter()
            .take((name_size - 2) as usize)
            .fold(1, |mut acc, &b| {
                if acc <= 0 {
                    acc = b;
                } else {
                    acc -= 1;
                    if b.is_ascii_alphanumeric() || b.is_ascii_punctuation() {
                        name.push(b as char);
                    } else { name.push('.')};
                };
                acc
            });
            // .for_each(|c| name.push(*c as char));

        // println!("name_size: {}, word_size: {}, offset: {}", name_size, word_size, request.len());

        // println!("Recived\n\n {:x?}", buff);

        Ok(name)
    }
}
