mod net;
use net::nbns::NbnsRequest;
use net::mdns::MdnsQuery;

#[derive(Debug)]
pub enum QueryError {
    ParseAddress,
    Network,
    NoAnswer,
    InvalidResponse,
}
impl std::error::Error for QueryError {}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "query error {}", match self {
            QueryError::ParseAddress => "ParseAddress",
            QueryError::Network => "Network",
            QueryError::NoAnswer => "NoAnswer",
            QueryError::InvalidResponse => "InvalidResponse"
        })
    }
}

pub fn ask(addr: &str) -> Result<String, QueryError> {
    // NbnsRequest::send(addr)
    MdnsQuery::send(addr)
}
