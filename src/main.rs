use blazinit::{cli::Cli, logging, run};
use clap::Parser;

fn main() {
    logging::init_logger();
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
