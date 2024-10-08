use std::sync::{Arc, Mutex};
use std::net::IpAddr;
use net::{QueryResult, nbns::NbnsQuery, mdns::MdnsQuery};
use clap::Parser;
use utils::AppendNewline;

mod net;
mod utils;

#[derive(Parser, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Target to ask hostname, can be
    /// address (192.168.1.100) or range (192.168.1.0/24)
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

#[derive(Debug, Clone, Copy)]
pub enum AppError {
    ParseAddress,
    ParseAddressesRange,
    SocketCreate,
    SocketConnect,
    SocketSend,
    SocketTimeout,
    InvalidResponseNbns,
    InvalidResponseMdns,
    InvalidResponses,
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
            AppError::InvalidResponseNbns => "recived invalid Nbns response",
            AppError::InvalidResponseMdns => "recived invalid mDNS response",
            AppError::InvalidResponses => "recived multiple invalid responses",
            AppError::ScanError => "errors occurred while scanning range of addresses",
            AppError::Ipv6 => "IPv6 is not supported yet",
        })
    }
}

/// When the program is run with `--wait` flag, it doesn't output immediately and stores everything in
/// `OutputBuffer`.
/// It's a `String` contained in `Arc<Mutex>`, so it can be shared between async threads.
#[derive(Clone)]
struct OutputBuffer (
    Arc<Mutex<String>>
);
impl OutputBuffer {
    fn new(ip_addr_type: IpAddr, args: &Args) -> Self {
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

/// Main struct. Contains `Args` and `OutputBuffer`.
/// `ask` and `ask_multiple` will ask for hostnames and domain name and output it to STDOUT or
/// `OutputBuffer` when `--wait` option is set.
/// On `drop` will print `OutputBuffer`, if should.
struct App {
    args: Args,
    output_buffer: OutputBuffer,
}
impl App {
    fn new(args: Args) -> Result<Self, AppError> {
        let ip_addr_type: IpAddr = if args.target.contains('/') {
            args.target.parse::<ipnet::IpNet>()
                .map_err(|_| AppError::ParseAddressesRange)?
                .addr()
        } else {
            args.target.parse()
                .map_err(|_| AppError::ParseAddress)?
        };
        if let Some(new_timeout) = args.timeout {
            match new_timeout {
                0 ..= net::TOO_LOW_TIMEOUT_WARNING_MS => { eprintln!("The selected timeout may be too low for reciving answers")},
                net::TOO_BIG_TIMEOUT_WARNING_MS..     => { eprintln!("The selected timeout may be too big and scanning may be slow")},
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

    // borrowing rules doesn't allow moving self to aync block, so query_and_out supossed to be
    // wrapped in ask and ask_multiple functions.
    // Instead of copying self, ask_multiple copies Arc<Mutex<OutputBuffer>> and passes it to query_and_out.
    fn query_and_out(addr: IpAddr, mut out: OutputBuffer, wait: bool, verbose: bool) -> Result<(), AppError> {

        if addr.is_ipv6() { return Err(AppError::Ipv6) };

        let mut result = QueryResult::new(addr);
        let mut errors: Vec<AppError> = Vec::new();

        if addr.is_ipv4() { // Nbns doesn't support IPv6
            match NbnsQuery::send(addr) {
                Ok(Some(ans)) => {
                    for i in ans {
                        result.push_hostname(i);
                    };
                },
                Ok(None) => {}
                Err(e) => {
                    errors.push(e);
                },
            };
        }

        match MdnsQuery::send(addr) {
            Ok(Some(ans)) => {
                result.set_domain_name(ans.to_string());
            },
            Ok(None) => {},
            Err(e) => {
                errors.push(e);
            }
        }

        if !result.is_empty() {
            if verbose {
                out.write(&result.verbose_entry(), wait);
            } else {
                out.write(&result.table_row(), wait);
            }
        }

        if !errors.is_empty() {
            if errors.len() == 1 {
                return Err(*errors.first().unwrap());
            } else {
                return Err(AppError::InvalidResponses)
            }
        }

        Ok(())
    }
    /// Asks `addr` for any names and outputs them to STDOUT or to `OutputBuffer` when `self.wait` is true.
    /// If `self.verbose` is true, will verbosely format entry.
    /// When querying resulted an error, will return `AppError`.
    fn ask(&mut self, addr: IpAddr) -> Result<(), AppError> {
        Self::query_and_out(addr, self.output_buffer.clone(), self.args.wait, self.args.verbose)
    }
    /// Asynchronously asks every host in `addr_range` and outputs results to STDOUT or
    /// `OutputBuffer` if `self.wait` is true.
    /// When any of querying resulted an error, will print address and error to STDERR and return `AppError::ScanError`
    fn ask_multiple(&mut self, addr_range: ipnet::IpNet) -> Result<(), AppError> {
        if addr_range.addr().is_ipv6() { return Err(AppError::ScanError) };
        // the only case where async is needed is in this function
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let errors: Arc<Mutex<Vec<(IpAddr, AppError)>>> = Arc::new(Mutex::new(Vec::new()));

        for addr in addr_range.hosts() {
            let b = self.output_buffer.clone(); // Rc<Mutex>
            let wait = self.args.wait;
            let verbose = self.args.verbose;
            let err_vec = errors.clone();
            rt.spawn_blocking(move || {
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
        let addr: IpAddr = app.target()
            .parse()
            .map_err(|_| AppError::ParseAddress)?;

        app.ask(addr)?;
    } else {
        let range: ipnet::IpNet = app.target()
            .parse()
            .map_err(|_| AppError::ParseAddressesRange)?;

        app.ask_multiple(range.into())?;
    }

    Ok(())
}
