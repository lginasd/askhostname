use clap::Parser;
use askhostname::run;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    target: String,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();

    match run(&args.target) {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("askhostname: {}", e);
            return std::process::ExitCode::FAILURE;
        },
    }
}
