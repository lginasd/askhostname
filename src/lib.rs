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
// NODE STATUS REQUEST
impl NdnsRequest {
    const PORT: u16 = 137; // NetBIOS port
    const SIZE: usize = std::mem::size_of::<NdnsRequest>();
    const TIMEOUT_MS: u64 = 1500;

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
    pub fn as_slice(&self) -> &[u8; Self::SIZE] {
        unsafe {
            &*(self as *const NdnsRequest as *const [u8; Self::SIZE])
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
impl std::error::Error for QuerryError {}
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

pub fn ask(addr: &str) -> Result<String, QuerryError> {
    let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("Failed to create socket");

    let remote: SocketAddr = match format!("{}:{}", addr, NdnsRequest::PORT).parse() {
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

    let mut tmp_buff: [std::mem::MaybeUninit<u8>; 256] = [std::mem::MaybeUninit::new(0); 256];
    let buff: Vec<u8>;
    sock.set_read_timeout(Some(std::time::Duration::from_millis(NdnsRequest::TIMEOUT_MS))).unwrap();

    if let Err(e) = sock.recv_from(&mut tmp_buff) {
        eprintln!("Failed to recive message: {}", e);
        return Err(QuerryError::NoAnswer);
    };

    // tmp_buff is always initialized
    unsafe { buff = tmp_buff.iter().map(|x| x.assume_init()).collect(); }

    // println!("Recived\n\n {:x?}", buff);

    // response contain request + time to live [0u8; 4] + answer
    // the next two bytes correspond to the answer size, followed by a one byte count of names
    // next chunks of 18 bytes represent name [u8; 16] + permanent node flags [u8; 2]
    // other data is ignored
    let (_, response) = buff.split_at(NdnsRequest::SIZE + 4);
    let data_size: u16 = ((response[0] as u16) << 8) + response[1] as u16;
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
