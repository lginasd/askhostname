use clap::Parser;
use askhostname::ask;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    target: String,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();

    let res = ask(&args.target);
    match res {
        Ok(hostname) => {
            println!("{}: {}", &args.target, hostname);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to get hostname: {}", e);
            std::process::ExitCode::FAILURE
        }
    }
}
