mod net;
use net::nbns::NbnsQuery;
use net::mdns::MdnsQuery;

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

pub fn run(addr: &str) -> Result<Option<String>, QueryError> {
    let mut res = String::new();

    let addr: std::net::IpAddr = addr.parse().expect("Failed to parse address");

    if let Some(ans) = NbnsQuery::send(addr)? {
        res.push_str(&ans);
    };
    if let Some(ans) = MdnsQuery::send(addr)? {
        res.push(' ');
        res.push_str(&ans);
    };

    Ok(Some(res))
}
