use askhostname::{run, Args};
use clap::Parser;

fn main() -> std::process::ExitCode {

    match run(Args::parse()) {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("askhostname: {}", e);
            std::process::ExitCode::FAILURE
        },
    }
}
