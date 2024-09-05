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

/// When the program is run with --wait flag, it doesn't output immediatly and stores everything in
/// `OutputBuffer`.
/// It's a `String` contained in `Arc<Mutex>`, so it can be shared between async threads.
#[derive(Clone)]
struct OutputBuffer (
    Arc<Mutex<String>>
);
impl OutputBuffer {
    fn new(ip_addr_type: std::net::IpAddr, args: &Args) -> Self {
        let mut s = Self { 0: Arc::new(Mutex::new(String::new())) };
        if !args.quiet {
            s.write(&QueryResult::table_head(&ip_addr_type), args.wait);
        }
        s
    }
    fn write(&mut self, s: &str, wait: bool) {
        if wait {
            let mut b = self.0.lock().unwrap();
            b.push_str(s);
            b.push('\n');
        } else {
            println!("{}", s);
        }
    }
}

struct App {
    args: Args,
    output_buffer: OutputBuffer,
}
impl App {
    fn new(args: Args) -> Self {
        let ip_addr_type: std::net::IpAddr;
        if args.target.contains('/') {
            ip_addr_type = args.target.parse::<ipnet::IpNet>()
                .unwrap()
                .addr();
        } else {
            ip_addr_type = args.target.parse().unwrap()
        }

        App {
            output_buffer: OutputBuffer::new(ip_addr_type, &args),
            args,
        }
    }
    fn target(&self) -> &str {
        &self.args.target
    }

    fn query_and_out(addr: std::net::IpAddr, mut out: OutputBuffer, wait: bool) -> Result<(), QueryError> {

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
            out.write(&result.table_row(), wait);
        }

        Ok(())
    }
    fn ask(&mut self, addr: std::net::IpAddr) -> Result<(), QueryError> {
        Self::query_and_out(addr, self.output_buffer.clone(), self.args.wait)
    }
    fn ask_multiple(&mut self, addr_range: ipnet::IpNet) -> Result<(), QueryError> {
        // the only case where async is needed is in this function
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        for addr in addr_range.hosts() {
            let b = self.output_buffer.clone(); // Rc<Mutex>
            let wait = self.args.wait;
            rt.spawn(async move {
                Self::query_and_out(addr, b, wait)
            });
        };

        Ok(())
    }
}
impl Drop for App {
    fn drop(&mut self) {
        if self.args.wait {
            println!("{}", self.output_buffer.0.lock().unwrap())
        }
    }
}

pub fn run(args: Args) -> Result<(), QueryError> {

    let mut app = App::new(args);

    if !app.target().contains('/') {
        let addr: std::net::IpAddr = app.target()
            .parse()
            .map_err(|_| QueryError::ParseAddress)?;

        app.ask(addr)?;
    } else {
        let range: Ipv4Net = app.target()
            .parse()
            .map_err(|_| QueryError::ParseAddressesRange)?;

        app.ask_multiple(range.into())?;
    }

    Ok(())
}
