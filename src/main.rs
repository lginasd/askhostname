use clap::Parser;
use askhostname::run;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    target: String,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();

    let res = run(&args.target);
    match res {
        Ok(hostname) => {
            if hostname.is_some() {
                println!("{}: {}", &args.target, hostname.unwrap());
            };
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to get hostname: {}", e);
            std::process::ExitCode::FAILURE
        }
    }
}
