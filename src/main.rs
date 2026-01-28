use blazinit::{cli::Cli, run};
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
