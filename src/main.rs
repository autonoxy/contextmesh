use clap::Parser;
use commands::Cli;
use env_logger::Env;

mod commands;
mod indexer;
mod parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Cli::parse();
    commands::run_command(args)
}
