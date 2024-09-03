mod net;
use net::nbns::{NbnsQuery, NbnsAnswer};
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

struct QueryResult {
    ip_addr: std::net::IpAddr,
    host_names: Vec<NbnsAnswer>,
    domain_name: String
}
impl QueryResult {
    const PADDING_IP4: usize = 16;
    const PADDING_IP6: usize = 36;
    const PADDING_HOSTNAME: usize = 16;
    const PADDING_DOMAIN_NAME: usize = 20;

    fn new(ip_addr: std::net::IpAddr) -> Self {
        QueryResult {
            ip_addr,
            host_names: Vec::new(),
            domain_name: String::new()
        }
    }
    fn is_empty(&self) -> bool {
        self.host_names.is_empty() && self.domain_name.is_empty()
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
    fn table_row(&self) -> String {
        if self.is_empty() { return "".to_string() };

        Self::format_row(
            &self.ip_addr,
            &self.host_names.first().unwrap_or(&net::nbns::NbnsAnswer::None).to_string(),
            &self.domain_name,
            self.ip_addr.is_ipv6(),
        )
    }
    fn table_head(addr: &std::net::IpAddr) -> String {
        Self::format_row("IP address", "Hostname", "Domain name", addr.is_ipv6())
    }
}

pub fn run(addr: &str) -> Result<(), QueryError> {

    let addr: std::net::IpAddr = addr.parse().map_err(|_| QueryError::ParseAddress)?;

    let res = query(addr)?;

    println!("{}", QueryResult::table_head(&addr));

    println!("{}", res.table_row());

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
