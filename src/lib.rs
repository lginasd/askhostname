mod nbns;
mod mdns;
use nbns::NbnsRequest;
use mdns::MdnsQuerry;

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
    // NbnsRequest::send(addr)
    MdnsQuerry::send(addr)
}
