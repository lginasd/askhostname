// Multicast DNS
// https://datatracker.ietf.org/doc/html/rfc6762

use std::net::SocketAddr;
use socket2::{Socket, Domain, Type, Protocol};
use super::QuerryError;

#[repr(C)]
pub struct MdnsQuerry {
    // header
    trans_id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,

    qname: [u8; 28],
    qtype: u16,
    qclass: u16 // first bit is UNICAST-RESPONSE flag for QU (querry unicast), which desires
                // unicast respose back to the host
}
// Unicast direct reverse DNS lookup querry with unicast response directly to the host
impl MdnsQuerry {
    pub const PORT: u16 = 5353;
    pub const SIZE: usize = std::mem::size_of::<MdnsQuerry>();
    pub const TIMEOUT_MS: u64 = 1500;

    fn new() -> Self {
        let mut question = [0x41u8; 28];
        question[0]  = 3;
        question[1]  = 0x31;
        question[2]  = 0x30;
        question[3]  = 0x30;
        question[4]  = 1;
        question[5]  = 0x38;
        question[6]  = 3;
        question[7]  = 0x31;
        question[8]  = 0x36;
        question[9]  = 0x38;
        question[10] = 3;
        question[11] = 0x31;
        question[12] = 0x39;
        question[13] = 0x32;
        question[14] = 7;
        question[15] = 0x69;
        question[16] = 0x6e;
        question[17] = 0x2d;
        question[18] = 0x61;
        question[19] = 0x64;
        question[20] = 0x64;
        question[21] = 0x72;
        question[22] = 4;
        question[23] = 0x61;
        question[24] = 0x72;
        question[25] = 0x70;
        question[26] = 0x61;
        question[27] = 0;

        MdnsQuerry {
            trans_id: 0x0000u16.to_be(),
            flags: 0u16.to_be(),
            qdcount: 1u16.to_be(),
            ancount: 0u16.to_be(),
            nscount: 0u16.to_be(),
            arcount: 0u16.to_be(),

            qname: question,
            qtype: 0x000cu16.to_be(),
            qclass: 0x0001u16.to_be()
        }
    }
    pub fn as_slice(&self) -> &[u8; Self::SIZE] {
        unsafe {
            &*(self as *const Self as *const [u8; Self::SIZE])
        }
    }

    pub fn send(addr: &str) -> Result<String, QuerryError> {
        let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("Failed to create socket");

        let remote: SocketAddr = match format!("{}:{}", addr, Self::PORT).parse() {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Failed to parse target IP: {e}");
                return Err(QuerryError::ParseAddress);
            }
        };

        // send from 5353 port. DO NOT USE in one-shot multicast querries
        // let src: SocketAddr = "0.0.0.0:5353".parse().unwrap();
        // let _ = sock.bind(&src.into()).expect("Failed to bind port");

        let request = Self::new();
        if let Err(err) = sock.send_to(request.as_slice(), &remote.into()) {
            eprintln!("Failed to send request {}", err);
            return Err(QuerryError::Network);
        }

        sock.connect(&remote.into()).expect("Failed to initiate the connection");

        let mut tmp_buff: [std::mem::MaybeUninit<u8>; 256] = [std::mem::MaybeUninit::new(0); 256];
        let buff: Vec<u8>;
        sock.set_read_timeout(Some(std::time::Duration::from_millis(Self::TIMEOUT_MS))).unwrap();

        if let Err(e) = sock.recv_from(&mut tmp_buff) {
            eprintln!("Failed to recive message: {}", e);
            return Err(QuerryError::NoAnswer);
        };

        // tmp_buff is always initialized
        unsafe { buff = tmp_buff.iter().map(|x| x.assume_init()).collect(); }

        println!("Recived\n\n {:x?}", buff);

        Ok(String::from("nah"))

    }
}
