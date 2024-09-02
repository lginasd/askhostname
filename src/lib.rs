mod net;
use net::nbns::{NbnsQuery, NbnsAnswer};
use net::mdns::MdnsQuery;
use tabled::{builder::Builder, settings::Style};

#[derive(Debug)]
pub enum QueryError {
    ParseAddress,
    Network,
    InvalidResponse,
}
impl std::error::Error for QueryError {}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "query error {}", match self {
            QueryError::ParseAddress => "ParseAddress",
            QueryError::Network => "Network",
            QueryError::InvalidResponse => "InvalidResponse"
        })
    }
}

struct QueryResult {
    ip_addr: std::net::IpAddr,
    host_names: Vec<NbnsAnswer>,
    domain_name: String
}
impl QueryResult {
    fn new(ip_addr: std::net::IpAddr) -> Self {
        QueryResult {
            ip_addr,
            host_names: Vec::new(),
            domain_name: String::new()
        }
    }
}

pub fn run(addr: &str) -> Result<(), QueryError> {

    let addr: std::net::IpAddr = addr.parse().map_err(|_| QueryError::ParseAddress)?;

    let res = query(addr)?;

    let mut tbuilder = Builder::new();
    tbuilder.push_record(["IP address", "Hostname", "Domain name"]);
    tbuilder.push_record([
        addr.to_string(),
        res.host_names.first().unwrap_or(&net::nbns::NbnsAnswer::None).to_string(),
        res.domain_name,
    ]);

    let t = tbuilder.build()
        .with(Style::empty())
        .to_string();

    println!("{}", t);

    Ok(())
}

fn query(addr: std::net::IpAddr) -> Result<QueryResult, QueryError> {

    // TODO: send arp first

    let mut result = QueryResult::new(addr);

    if addr.is_ipv4() { // Nbns doesn't support IPv6
        if let Some(ans) = NbnsQuery::send(addr)? {
            for i in ans {
                result.host_names.push(i);
            };
        };
    }

    if let Some(ans) = MdnsQuery::send(addr)? {
        result.domain_name = ans;
    };

    Ok(result)
}
