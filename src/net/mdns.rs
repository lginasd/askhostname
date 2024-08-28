// Multicast DNS
// https://www.rfc-editor.org/rfc/rfc6762.html

use crate::QueryError;
use crate::net::{DnsHeader, query};
use std::net::IpAddr;

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

        // name is encoded as ASCII octets preceded by their amount, ended with NULL-terminator (0x00)
        // for example domain "abc.com" would be 0x03 0x41 0x42 0x43 0x03 0x43 0x6f 0x6d 0x00
        // for example addres 127.0.0.1 is 0x03 0x31 0x32 0x37 0x01 0x30 0x01 0x30 0x01 0x31 0x00
        // for reverse DNS lookup address is reversed and represented like char arrays + .in-addr.arpa
        // note that there is no '.' (0x2e), instead amount of octets (size of word)
        match ip {
            IpAddr::V4(a) => {
                let octets: Vec<String> = a.octets().into_iter().map(|x| x.to_string()).collect();
                for octet in octets.iter().rev() {
                    question.push(octet.len() as u8);
                    octet.chars().for_each(|c| { question.push(c as u8); });
                }
            },
            IpAddr::V6(_a) => {
                todo!()
            }
        };

        question.push(7); // size of "in-addr"
        "in-addr".chars().for_each(|c| question.push(c as u8));
        question.push(4); // size of "arpa"
        "arpa".chars().for_each(|c| question.push(c as u8));
        question.push(0);


        MdnsQuery {
            header: DnsHeader::new_mdns(),

            qname: question,
            qtype: 0x000cu16.to_be(), // PTR
            qclass: 0x0001u16.to_be() // IN (ARPA)
        }
    }
    fn header_as_slice(&self) -> &[u8; DnsHeader::SIZE] {
        unsafe {
            &*(self as *const Self as *const [u8; DnsHeader::SIZE])
        }
    }

    fn to_packet(&self) -> Vec<u8> {
        let mut tmp_vec: Vec<u8> = vec![];
        self.header_as_slice().map(|b| tmp_vec.push(b));
        self.qname.iter().for_each(|b| tmp_vec.push(*b));

        tmp_vec.push(self.qtype as u8);
        tmp_vec.push((self.qtype >> 8) as u8);

        tmp_vec.push(self.qclass as u8);
        tmp_vec.push((self.qclass >> 8) as u8);

        tmp_vec
    }

    pub fn send(addr: IpAddr) -> Result<Option<String>, QueryError> {
        let request = Self::new(addr).to_packet();

        let buff = query(addr, Self::PORT, &request)?;
        if buff.is_none() { return Ok(None) };
        let buff = buff.unwrap();

        // response contains request + response name [u8; 2] + response type [u8; 2] + cache flush [u8; 2] + time to live [u8; 4] + answer
        // so actual response is at buff[(request_size + 10)..]
        let (_, response) = buff.split_at(request.len() + 10);
        // the next two bytes correspond to the answer size
        let answer_size: u16 = ((response[0] as u16) << 8) | response[1] as u16;

        let mut name = String::new();

        // println!("Response: {:x?}", response);

        // name consists of the words (ASCII octet strings, preceded by their size)
        // for example "abc.com" would be 0x03 0x41 0x42 0x43 0x03 0x43 0x6f 0x6d 0x00
        // usualy the last word is "local"

        let mut word_size = response[2]; // size of first array of octets

        // start from the first array of characters
        response[3..].iter()
            .take((answer_size - 2) as usize) // ignore NULL-terminator
            .for_each(|&b| {
                if word_size == 0 {
                    word_size = b;
                    name.push('.');
                } else {
                    word_size -= 1;
                    if b.is_ascii_alphanumeric() || b.is_ascii_punctuation() {
                        name.push(b as char);
                    }
                };
            });

        // println!("answer_size: {}, word_size: {}, offset: {}", name_size, word_size, request.len());

        Ok(Some(name))
    }
}
