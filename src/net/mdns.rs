// Multicast DNS
// https://datatracker.ietf.org/doc/html/rfc6762

use std::net::{SocketAddr, IpAddr};
use socket2::{Socket, Domain, Type, Protocol};
use crate::QuerryError;
use crate::net::DnsHeader;
use crate::net::querry;

#[repr(C)]
pub struct MdnsQuerry {
    header: DnsHeader,

    // qname: [u8; 28],
    qname: Vec<u8>, // is dynamic size
    qtype: u16,
    qclass: u16 // first bit is UNICAST-RESPONSE flag for QU (querry unicast), which desires
                // unicast respose back to the host
}
// Unicast direct reverse DNS lookup querry with unicast response directly to the host
impl MdnsQuerry {
    pub const PORT: u16 = 5353;
    pub const SIZE: usize = std::mem::size_of::<MdnsQuerry>();

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

        // question[14] = 7;
        // question[15] = 0x69;
        // question[16] = 0x6e;
        // question[17] = 0x2d;
        // question[18] = 0x61;
        // question[19] = 0x64;
        // question[20] = 0x64;
        // question[21] = 0x72;
        // question[22] = 4;
        // question[23] = 0x61;
        // question[24] = 0x72;
        // question[25] = 0x70;
        // question[26] = 0x61;
        // question[27] = 0;

        MdnsQuerry {
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

    pub fn send(addr: &str) -> Result<String, QuerryError> {

        // let ip: SocketAddr = format!("{}:0", addr).parse().unwrap();
        // let ip = ip.ip();
        let ip = addr.parse().expect("Ip parse failed. MDNS send");
        let request = Self::new(ip).message();

        let buff = querry(addr, Self::PORT, &request)?;

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
