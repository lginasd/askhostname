pub mod nbns;
pub mod mdns;

use crate::QueryError;
use std::net::{UdpSocket, IpAddr};


pub const TIMEOUT_MS: std::time::Duration = std::time::Duration::from_millis(1500);
pub const RECV_BUFF_SIZE: usize = 256;

// DOMAIN NAMES - IMPLEMENTATION and SPECIFICATION  https://www.rfc-editor.org/rfc/rfc883
// DOMAIN NAMES - CONCEPTS AND FACILITIES           https://www.rfc-editor.org/rfc/rfc1034

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
            trans_id: 0x5021u16.to_be(), // TODO: randomize
            flags:    0u16.to_be(), // unicast
            qdcount:  1u16.to_be(),
            ancount:  0u16.to_be(),
            nscount:  0u16.to_be(),
            arcount:  0u16.to_be(),
        }
    }
    fn new_mdns() -> Self {
        Self {
            trans_id: 0u16.to_be(), // should be 0 for mdns
            flags:    0u16.to_be(), // unicast
            qdcount:  1u16.to_be(),
            ancount:  0u16.to_be(),
            nscount:  0u16.to_be(),
            arcount:  0u16.to_be(),
        }
    }
}

fn query(addr: IpAddr, port: u16, request: &[u8]) -> Result<Option<Vec<u8>>, QueryError> {
    let sock = UdpSocket::bind("0.0.0.0:0").expect("Failed to create socket");

    if let Err(_) = sock.connect((addr, port)) {
        return Err(QueryError::Network);
    }

    if sock.set_write_timeout(Some(TIMEOUT_MS)).is_err() { return Err(QueryError::Network) };
    if sock.set_read_timeout (Some(TIMEOUT_MS)).is_err() { return Err(QueryError::Network) };

    if let Err(err) = sock.send(request) {
        eprintln!("Failed to send request: {}", err);
        return Err(QueryError::Network);
    };

    let mut response = [0; RECV_BUFF_SIZE];
    if sock.recv(&mut response).is_err() {
        return Ok(None);
    };

    Ok(Some(response.to_vec()))
}
// old socket2 implementation
// fn query(addr: &str, port: u16, request: &[u8]) -> Result<Option<Vec<u8>>, QueryError> {
//     let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("Failed to create socket");
//
//     let remote: SocketAddr = match format!("{}:{}", addr, port).parse() {
//         Ok(a) => a,
//         Err(e) => {
//             eprintln!("Failed to parse target IP: {e}");
//             return Err(QueryError::ParseAddress);
//         }
//     };
//
//     if let Err(err) = sock.send_to(request, &remote.into()) {
//         eprintln!("Failed to send request {}", err);
//         return Err(QueryError::Network);
//     }
//
//     sock.connect(&remote.into()).expect("Failed to initiate the connection");
//
//     let mut tmp_buff: [std::mem::MaybeUninit<u8>; RECV_BUFF_SIZE] = [std::mem::MaybeUninit::new(0); RECV_BUFF_SIZE];
//     let buff: Vec<u8>;
//
//     sock.set_read_timeout(Some(TIMEOUT_MS)).unwrap();
//
//     if let Err(_) = sock.recv_from(&mut tmp_buff) {
//         return Ok(None);
//     };
//
//     // tmp_buff is always initialized
//     unsafe { buff = tmp_buff.iter().map(|x| x.assume_init()).collect(); }
//
//     Ok(Some(buff))
// }
