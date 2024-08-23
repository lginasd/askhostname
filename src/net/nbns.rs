// NetBIOS Name Service
// https://www.rfc-editor.org/rfc/rfc1002.html

use std::net::SocketAddr;
use socket2::{Socket, Domain, Type, Protocol};
use crate::QuerryError;
use crate::net::DnsHeader;
use crate::net::querry;

#[repr(C)]
pub struct NbnsRequest {
    header: DnsHeader,

    // question
    question: [u8; 34],
    qtype: u16,
    qclass: u16,
}
// NODE STATUS REQUEST
impl NbnsRequest {
    pub const PORT: u16 = 137; // NetBIOS port
    pub const SIZE: usize = std::mem::size_of::<NbnsRequest>();

    fn new() -> Self {
        let mut question = [0x41u8; 34];
        question[0]  = 0x20;
        question[1]  = 0x43;
        question[2]  = 0x4b;
        question[33] = 0;

        NbnsRequest {
            header: DnsHeader::new_nbns(),

            question,
            qtype: 0x0021u16.to_be(),
            qclass: 0x0001u16.to_be()
        }
    }
    pub fn as_slice(&self) -> &[u8; Self::SIZE] {
        unsafe {
            &*(self as *const Self as *const [u8; Self::SIZE])
        }
    }

    pub fn send(addr: &str) -> Result<String, QuerryError> {

        let request = Self::new();

        let buff = querry(addr, Self::PORT, request.as_slice())?;

        // println!("Recived\n\n {:x?}", buff);

        // response contain request + time to live [0u8; 4] + answer
        // the next two bytes correspond to the answer size, followed by a one byte count of names
        // next chunks of 18 bytes represent name [u8; 16] + permanent node flags [u8; 2]
        // other data is ignored
        let (_, response) = buff.split_at(Self::SIZE + 4);
        let data_size: u16 = ((response[0] as u16) << 8) | response[1] as u16;
        let names_count: u8 = response[2];

        let mut names = Vec::new();
        for chunk in response[3..data_size as usize].chunks(18).take(names_count as usize) {
            // [NAME + OPTIONAL_PADDING(0x20)]: [u8; 16] + FLAGS [u8; 2] on each 16 bytes chunk
            let name: String = chunk[..15].iter()
                .filter_map(|b| {
                    if b.is_ascii_alphanumeric() || b.is_ascii_punctuation() { Some(*b as char) }
                    else { None }
                }).collect();

            names.push(name)
        };
        if names.is_empty() { return Err(QuerryError::InvalidResponse) };
        // println!("Debug: names is {:?}", names);

        // For now return only first name, as it's the most reliable. Maybe return all later, if output
        // should be verbose
        Ok(names.get(0).unwrap().to_string())

    }
}
