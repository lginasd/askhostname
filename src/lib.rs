use std::sync::{Arc, Mutex};
mod net;
use net::QueryResult;
use net::nbns::NbnsQuery;
use net::mdns::MdnsQuery;
use ipnet::Ipv4Net;
use clap::Parser;
mod utils;
use utils::AppendNewline;

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

    /// Timeout in milliseconds
    #[arg(short, long)]
    timeout: Option<u64>,
}

#[derive(Debug)]
pub enum AppError {
    ParseAddress,
    ParseAddressesRange,
    SocketCreate,
    SocketConnect,
    SocketSend,
    SocketTimeout,
    InvalidResponse,
    ScanError,
    Ipv6,
}
impl std::error::Error for AppError {}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", match self {
            AppError::ParseAddress => "failed to parse address",
            AppError::ParseAddressesRange => "failed to parse address range",
            AppError::SocketCreate => "failed to create socket",
            AppError::SocketConnect => "connection with remote host failed",
            AppError::SocketSend => "failed to send request",
            AppError::SocketTimeout => "invalid socket timeout",
            AppError::InvalidResponse => "recived invalid response",
            AppError::ScanError => "errors occurred while scanning range of addresses",
            AppError::Ipv6 => "IPv6 is not supported yet",
        })
    }
}

/// When the program is run with --wait flag, it doesn't output immediately and stores everything in
/// `OutputBuffer`.
/// It's a `String` contained in `Arc<Mutex>`, so it can be shared between async threads.
#[derive(Clone)]
struct OutputBuffer (
    Arc<Mutex<String>>
);
impl OutputBuffer {
    fn new(ip_addr_type: std::net::IpAddr, args: &Args) -> Self {
        let mut s = Self ( Arc::new(Mutex::new(String::new())) );
        if !(args.quiet || args.verbose) {
            s.write(&QueryResult::table_head(&ip_addr_type), args.wait);
        }
        s
    }
    fn write(&mut self, s: &str, wait: bool) {
        if wait {
            let mut b = self.0.lock().unwrap();
            b.push_str(s);
            b.new_line();
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
    fn new(args: Args) -> Result<Self, AppError> {
        let ip_addr_type: std::net::IpAddr = if args.target.contains('/') {
            args.target.parse::<ipnet::IpNet>()
                .map_err(|_| AppError::ParseAddressesRange)?
                .addr()
        } else {
            args.target.parse()
                .map_err(|_| AppError::ParseAddress)?
        };
        if let Some(new_timeout) = args.timeout {
            match new_timeout {
                0..=100 => { eprintln!("The selected timeout may be too low for reciving answers")},
                5000.. =>  { eprintln!("The selected timeout may be too big and scanning may be slow")},
                _ => {}
            }
            if let Err(e) = net::set_timeout_from_millis(new_timeout) {
                return Err(e)
            };
        }

        Ok(App {
                output_buffer: OutputBuffer::new(ip_addr_type, &args),
                args,
            })
    }
    fn target(&self) -> &str {
        &self.args.target
    }

    fn query_and_out(addr: std::net::IpAddr, mut out: OutputBuffer, wait: bool, verbose: bool) -> Result<(), AppError> {

        if addr.is_ipv6() { return Err(AppError::Ipv6) };

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
            if verbose {
                out.write(&result.verbose_entry(), wait);
            } else {
                out.write(&result.table_row(), wait);
            }
        }

        Ok(())
    }
    fn ask(&mut self, addr: std::net::IpAddr) -> Result<(), AppError> {
        Self::query_and_out(addr, self.output_buffer.clone(), self.args.wait, self.args.verbose)
    }
    fn ask_multiple(&mut self, addr_range: ipnet::IpNet) -> Result<(), AppError> {
        // the only case where async is needed is in this function
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let errors: Arc<Mutex<Vec<(std::net::IpAddr, AppError)>>> = Arc::new(Mutex::new(Vec::new()));

        for addr in addr_range.hosts() {
            let b = self.output_buffer.clone(); // Rc<Mutex>
            let wait = self.args.wait;
            let verbose = self.args.verbose;
            let err_vec = errors.clone();
            rt.spawn(async move { // NOTE: will ignore all errors
                if let Err(e) = Self::query_and_out(addr, b, wait, verbose) {
                    err_vec.lock().unwrap().push((addr, e));
                };
            });
        };

        let errors = errors.lock().unwrap();
        if !errors.is_empty() {
            for (addr, err) in errors.iter() {
                eprintln!("Error for {}: {}", addr, err);
            }
            return Err(AppError::ScanError);
        }

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

pub fn run(args: Args) -> Result<(), AppError> {

    let mut app = App::new(args)?;

    if !app.target().contains('/') {
        let addr: std::net::IpAddr = app.target()
            .parse()
            .map_err(|_| AppError::ParseAddress)?;

        app.ask(addr)?;
    } else {
        let range: Ipv4Net = app.target()
            .parse()
            .map_err(|_| AppError::ParseAddressesRange)?;

        app.ask_multiple(range.into())?;
    }

    Ok(())
}
