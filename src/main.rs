use clap::Parser;
use commands::Cli;

mod commands;
mod indexer;
mod parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    commands::run_command(args)
}

