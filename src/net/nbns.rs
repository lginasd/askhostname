// NetBIOS Name Service
// https://www.rfc-editor.org/rfc/rfc1002.html

use crate::AppError;
use crate::net::{DnsHeader, MacAddress, query};
use std::net::IpAddr;

#[repr(C)]
pub struct NbnsQuery {
    header: DnsHeader,

    // question
    question: [u8; 34],
    qtype: u16,
    qclass: u16,
}
// NODE STATUS REQUEST
impl NbnsQuery {
    pub const PORT: u16 = 137; // NetBIOS port
    pub const SIZE: usize = std::mem::size_of::<NbnsQuery>();
    const MIN_RESPONSE_SIZE: usize = 32;

    fn new() -> Self {
        // same question is send by nbtscan and nbstat.exe
        let mut question = [0x41u8; 34];
        question[0]  = 0x20;
        question[1]  = 0x43;
        question[2]  = 0x4b;
        question[33] = 0;

        NbnsQuery {
            header: DnsHeader::new_nbns(),

            question,
            qtype: 0x0021u16.to_be(), // NetBIOS NODE STATUS Resource Record
            qclass: 0x0001u16.to_be()
        }
    }
    fn as_slice(&self) -> &[u8; Self::SIZE] {
        unsafe {
            &*(self as *const Self as *const [u8; Self::SIZE])
        }
    }

    pub fn send(addr: IpAddr) -> Result<Option<Vec<NbnsAnswer>>, AppError> {

        let request = Self::new();

        let buff = query(addr, Self::PORT, request.as_slice())?;
        if buff.is_none() { return Ok(None) };
        let buff = buff.unwrap();
        if buff.len() <= Self::SIZE + Self::MIN_RESPONSE_SIZE { return Err(AppError::InvalidResponseNbns) };

        // response contains request + time to live [0u8; 4] + answer
        // the next two bytes correspond to the answer size, followed by a one byte count of names
        // next chunks of 18 bytes represent name [u8; 16] + permanent node flags [u8; 2]
        // array of names is followed by MAC address [u8; 6]
        // other data is ignored
        let (_, mut response) = buff.split_at(Self::SIZE + 4);
        let data_size: u16 = ((response[0] as u16) << 8) | response[1] as u16;
        let names_count: u8 = response[2];
        response = &response[3..]; // move slice start to exclude previously obtained data

        let mut names = Vec::new();
        for chunk in response[..data_size as usize].chunks(18).take(names_count as usize) {
            // [NAME + OPTIONAL_PADDING(0x20)]: [u8; 15] + SERVICE: u8 + FLAGS: [u8; 2] on each 18 bytes chunk
            let name: String = chunk[..=14].iter()
                .filter_map(|b| {
                    if b.is_ascii_alphanumeric() || b.is_ascii_punctuation() { Some(*b as char) }
                    else { None }
                }).collect();
            let service = chunk[15];
            let flags = chunk[16]; // chunk[17] is reserved and always should be zero

            names.push(match flags {
                f if f & 0x82 == 0x82 => NbnsAnswer::PermanentGroup((name, service)),
                f if f & 0x02 != 0 => NbnsAnswer::Permanent((name, service)),
                f if f & 0x80 != 0 => NbnsAnswer::Group((name, service)),
                _ => NbnsAnswer::Unique((name, service)),
            })
        };
        let raw_mac = &response[names_count as usize * 18 .. names_count as usize * 18 + 6];
        if let Some(mac) = MacAddress::from_bytes(raw_mac) {
            names.push(NbnsAnswer::Mac(mac));
        }

        if names.is_empty() { return Err(AppError::InvalidResponseNbns) };
        Ok(Some(names))
    }
}

pub enum NbnsAnswer {
    Unique((String, u8)),
    Group((String, u8)),
    Permanent((String, u8)),
    PermanentGroup((String, u8)),
    Mac(MacAddress),
}
impl std::fmt::Display for NbnsAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NbnsAnswer::Unique((name, _))         => { write!(f, "{}", name) },
            NbnsAnswer::Group((name, _))          => { write!(f, "{}", name) },
            NbnsAnswer::Permanent((name, _))      => { write!(f, "{}", name) },
            NbnsAnswer::PermanentGroup((name, _)) => { write!(f, "{}", name) },
            NbnsAnswer::Mac(_)                    => { write!(f, "") },
        }
    }
}
impl std::fmt::Debug for NbnsAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NbnsAnswer::Unique((name, service))         => { write!(f, "{} Service: {:x}", name, service) },
            NbnsAnswer::Group((name, service))          => { write!(f, "{} (Group) Service: {:x}", name, service) },
            NbnsAnswer::Permanent((name, service))      => { write!(f, "{} (Permanent name) Service: {:x}", name, service) },
            NbnsAnswer::PermanentGroup((name, service)) => { write!(f, "{} (Permanent group) Service: {:x}", name, service) },
            NbnsAnswer::Mac(mac)                        => { write!(f, "MAC address: {}", mac) }
        }
    }
}
