use clap::Parser;
use commands::Cli;

mod commands;
mod cache;
mod parser;
mod indexer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    commands::run_command(args)
}