use clap::Parser;
use commands::Cli;
use env_logger::Env;

mod commands;
mod errors;
mod indexer;
mod parser;
mod symbol;
mod utils;

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Cli::parse();
    if let Err(e) = commands::run_command(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
