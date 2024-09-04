use std::sync::{Arc, Mutex};
mod net;
use net::QueryResult;
use net::nbns::NbnsQuery;
use net::mdns::MdnsQuery;
use ipnet::Ipv4Net;
use clap::Parser;

#[derive(Parser, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Target to ask hostname (can be range with CIDR notation)
    target: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Quieter output
    #[arg(short, long)]
    quiet: bool,

    /// Wait for all answers, and then print them at once
    #[arg(short, long)]
    wait: bool,
}

#[derive(Debug)]
pub enum QueryError {
    ParseAddress,
    ParseAddressesRange,
    Network,
    InvalidResponse,
}
impl std::error::Error for QueryError {}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "query error {}", match self {
            QueryError::ParseAddress => "ParseAddress",
            QueryError::ParseAddressesRange => "ParseAddressesRange",
            QueryError::Network => "Network",
            QueryError::InvalidResponse => "InvalidResponse",
        })
    }
}

#[derive(Clone)]
struct App {
    args: Args,
    output_buffer: Arc<Mutex<String>>, // If shouldn't print immediately
}
impl App {
    fn new(args: Args) -> Self {
        App {
            args,
            output_buffer: Arc::new(Mutex::new(String::new())),
        }
    }
    fn target(&self) -> &str {
        &self.args.target
    }
    fn write(&mut self, s: &str) {
        if self.args.wait {
            let mut b = self.output_buffer.lock().unwrap();
            b.push_str(s);
            b.push('\n'); // not very cross-platform
        } else {
            println!("{}", s);
        }
    }
    fn outbuff(&self) {
        if self.args.wait {
            println!("{}", &self.output_buffer.lock().unwrap());
        }
    }

    fn ask(&mut self, addr: std::net::IpAddr) -> Result<(), QueryError> {

        let mut result = QueryResult::new(addr);

        if addr.is_ipv4() { // Nbns doesn't support IPv6
            if let Some(ans) = NbnsQuery::send(addr)? {
                for i in ans {
                    result.push_hostname(i);
                };
            };
        }

        if let Some(ans) = MdnsQuery::send(addr)? {
            result.set_domain_name(ans);
        };

        if !result.is_empty() {
            self.write(&result.table_row());
        }

        Ok(())
    }
    fn ask_multiple(&mut self, addr_range: ipnet::IpNet) -> Result<(), QueryError> {
        // the only case where async is needed is in this function
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        for addr in addr_range.hosts() {
            let mut t = self.clone();
            rt.spawn(async move {
                if let Err(e) = t.ask(addr) {
                    return Err(e);
                } else { return Ok(()) }
            });
        };

        Ok(())
    }
}

pub fn run(args: Args) -> Result<(), QueryError> {

    let mut app = App::new(args);

    if !app.target().contains('/') {
        let addr: std::net::IpAddr = app.target().parse().map_err(|_| QueryError::ParseAddress)?;

        app.write(&QueryResult::table_head(&addr));

        app.ask(addr)?;
    } else {
        let range: Ipv4Net = app.target().parse().map_err(|_| QueryError::ParseAddressesRange)?;

        app.write(&QueryResult::table_head(&range.addr().into()));

        app.ask_multiple(range.into())?;
    }
    app.outbuff();

    Ok(())
}
