pub mod check;
pub mod combine;
pub mod index;
pub mod print_index;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "contextmesh")]
#[command(about = "Tool for simplifying context gathering for llms")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Index {
        #[arg(short, long)]
        file: String,
        #[arg(short, long, default_value = "rust")]
        language: String,
    },
    Check {
        #[arg(short, long)]
        file: String,
    },
    Combine,
    PrintIndex,
}

pub fn run_command(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Commands::Index { file, language } => index::handle_index(&file, &language),
        Commands::Check { file } => check::handle_check(&file),
        Commands::Combine => combine::handle_combine(),
        Commands::PrintIndex => print_index::handle_print_index(),
    }
}
