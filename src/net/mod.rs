pub mod nbns;
pub mod mdns;
use std::net::SocketAddr;
use socket2::{Socket, Domain, Type, Protocol};
use crate::QuerryError;


pub const TIMEOUT_MS: std::time::Duration = std::time::Duration::from_millis(1500);

#[repr(C)]
struct DnsHeader {
    trans_id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}
impl DnsHeader {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    fn new_nbns() -> Self {
        Self {
            trans_id: 0x5021u16.to_be(),
            flags: (1u16 << 4).to_be(),
            qdcount: 1u16.to_be(),
            ancount: 0u16.to_be(),
            nscount: 0u16.to_be(),
            arcount: 0u16.to_be(),
        }
    }
    fn new_mdns() -> Self {
        Self {
            trans_id: 0x0000u16.to_be(),
            flags: 0u16.to_be(),
            qdcount: 1u16.to_be(),
            ancount: 0u16.to_be(),
            nscount: 0u16.to_be(),
            arcount: 0u16.to_be(),
        }
    }

    pub fn as_slice(&self) -> &[u8; Self::SIZE] {
        unsafe {
            &*(self as *const Self as *const [u8; Self::SIZE])
        }
    }
}

fn querry(addr: &str, port: u16, request: &[u8]) -> Result<Vec<u8>, QuerryError> {
    let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("Failed to create socket");

    let remote: SocketAddr = match format!("{}:{}", addr, port).parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to parse target IP: {e}");
            return Err(QuerryError::ParseAddress);
        }
    };

    if let Err(err) = sock.send_to(request, &remote.into()) {
        eprintln!("Failed to send request {}", err);
        return Err(QuerryError::Network);
    }

    sock.connect(&remote.into()).expect("Failed to initiate the connection");

    let mut tmp_buff: [std::mem::MaybeUninit<u8>; 256] = [std::mem::MaybeUninit::new(0); 256];
    let buff: Vec<u8>;
    sock.set_read_timeout(Some(TIMEOUT_MS)).unwrap();

    if let Err(e) = sock.recv_from(&mut tmp_buff) {
        eprintln!("Failed to recive message: {}", e);
        return Err(QuerryError::NoAnswer);
    };

    // tmp_buff is always initialized
    unsafe { buff = tmp_buff.iter().map(|x| x.assume_init()).collect(); }

    Ok(buff)
}
