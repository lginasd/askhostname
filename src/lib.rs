use std::net::SocketAddr;
use socket2::{Socket, Domain, Type, Protocol};

#[repr(C)]
struct NdnsRequest {
    // header
    trans_id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,

    // question
    question: [u8; 34],
    qtype: u16,
    qclass: u16,
}
const NDNS_REQUEST_SIZE: usize = std::mem::size_of::<NdnsRequest>();
// NODE STATUS REQUEST
impl NdnsRequest {
    pub fn new() -> Self {
        let mut question = [0x41u8; 34];
        question[0]  = 0x20;
        question[1]  = 0x43;
        question[2]  = 0x4b;
        question[33] = 0;
        NdnsRequest {
            trans_id: 0x5021u16.to_be(), // TODO: randomize
            flags: (1u16 << 4).to_be(),
            qdcount: 1u16.to_be(),
            ancount: 0u16.to_be(),
            nscount: 0u16.to_be(),
            arcount: 0u16.to_be(),

            question,
            qtype: 0x0021u16.to_be(),
            qclass: 0x0001u16.to_be()
        }
    }
    pub fn as_slice(&self) -> &[u8; NDNS_REQUEST_SIZE] {
        unsafe {
            &*(self as *const NdnsRequest as *const [u8; NDNS_REQUEST_SIZE])
        }
    }
}

#[derive(Debug)]
pub enum QuerryError {
    ParseAddress,
    Network,
    NoAnswer,
    InvalidResponse,
}
impl std::fmt::Display for QuerryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "querry error {}", match self {
            QuerryError::ParseAddress => "ParseAddress",
            QuerryError::Network => "Network",
            QuerryError::NoAnswer => "NoAnswer",
            QuerryError::InvalidResponse => "InvalidResponse"
        })
    }
}
impl std::error::Error for QuerryError {}

pub fn ask(addr: &str) -> Result<String, QuerryError> {
    let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("Failed to create socket");

    // NetBIOS port is 137
    let remote: SocketAddr = match format!("{}:137", addr).parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to parse target IP: {e}");
            return Err(QuerryError::ParseAddress);
        }
    };

    let request = NdnsRequest::new();
    if let Err(err) = sock.send_to(request.as_slice(), &remote.into()) {
        eprintln!("Failed to send request {}", err);
        return Err(QuerryError::Network);
    }

    sock.connect(&remote.into()).expect("Failed to initiate the connection");

    let mut raw: [std::mem::MaybeUninit<u8>; 256] = [std::mem::MaybeUninit::new(0); 256];
    let buff: Vec<u8>;
    sock.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
    if let Err(e) = sock.recv_from(&mut raw) {
        eprintln!("Failed to recive message: {}", e);
        return Err(QuerryError::NoAnswer);
    };

    // buffer is initialized
    unsafe { buff = raw.iter().map(|x| x.assume_init()).collect(); }

    // println!("Recived\n\n {:x?}", buff);

    let (_, response) = buff.split_at(54);
    // apparently not needed
    // let data_size: u16 = ((response[0] as u16) << 8) + response[1] as u16;
    // let names_count: u8 = response[2];

    // TODO: better parsing
    let idx = response.windows(2).position(|window| window == [0x84, 0x00]).expect("Invalid response");
    let (raw_name, _) = response.split_at(idx);

    let name = raw_name[3..].iter()
        .map_while(|b| {
            if b.is_ascii_alphanumeric() || b.is_ascii_punctuation() {
                Some(b)
            } else { None }
        })
        .fold(String::new(), |mut acc, b| { acc.push(*b as char); acc });

    Ok(name)
}
