pub mod nbns;
pub mod mdns;

use crate::QueryError;
use nbns::NbnsAnswer;
use std::net::{UdpSocket, IpAddr};


pub static mut TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);
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
            trans_id: rand::random::<u16>().to_be(),
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


pub struct QueryResult {
    ip_addr: std::net::IpAddr,
    host_names: Vec<NbnsAnswer>,
    domain_name: String
}
impl QueryResult {
    const PADDING_IP4: usize = 16;
    const PADDING_IP6: usize = 36;
    const PADDING_HOSTNAME: usize = 16;
    const PADDING_DOMAIN_NAME: usize = 20;

    pub fn new(ip_addr: std::net::IpAddr) -> Self {
        QueryResult {
            ip_addr,
            host_names: Vec::new(),
            domain_name: String::new()
        }
    }
    pub fn is_empty(&self) -> bool {
        self.host_names.is_empty() && self.domain_name.is_empty()
    }
    pub fn push_hostname(&mut self, hostname: NbnsAnswer) {
        self.host_names.push(hostname);
    }
    pub fn set_domain_name(&mut self, domain_name: String) {
        self.domain_name = domain_name;
    }

    // Different padding is needed for IPv4 and IPv6
    fn format_row<A, B, C>(a: A, b: B, c: C, is_ipv6: bool) -> String
    where A: std::fmt::Display, B: std::fmt::Display, C: std::fmt::Display
    {
        format!(
            "{:<ip_width$} {:<hostname_width$} {:<domain_name_width$}",
            a, b, c,

            ip_width = match is_ipv6 {
                false => Self::PADDING_IP4,
                true  => Self::PADDING_IP6,
            },
            hostname_width = Self::PADDING_HOSTNAME,
            domain_name_width = Self::PADDING_DOMAIN_NAME,
        )
    }
    pub fn table_row(&self) -> String {
        assert!(!self.is_empty());

        let hostname = match self.host_names.first() {
            Some(n) => n.to_string(),
            None => "-".to_string()
        };

        let domain_name = if !self.domain_name.is_empty() {
            self.domain_name.to_string()
        } else {
            "-".to_string()
        };

        Self::format_row(
            self.ip_addr,
            hostname,
            domain_name,
            self.ip_addr.is_ipv6(),
        )
    }
    pub fn table_head(addr: &std::net::IpAddr) -> String {
        Self::format_row("IP address", "Hostname", "Domain name", addr.is_ipv6())
    }
}

fn query(addr: IpAddr, port: u16, request: &[u8]) -> Result<Option<Vec<u8>>, QueryError> {
    let sock = UdpSocket::bind("0.0.0.0:0").expect("Failed to create socket");

    if let Err(_) = sock.connect((addr, port)) {
        return Err(QueryError::Network);
    }

    let timeout: std::time::Duration;
    unsafe {
        timeout = TIMEOUT;
    }
    if sock.set_write_timeout(Some(timeout)).is_err() { return Err(QueryError::Network) };
    if sock.set_read_timeout (Some(timeout)).is_err() { return Err(QueryError::Network) };

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
