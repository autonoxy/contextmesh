pub mod combine;
pub mod index;
pub mod print_index;

use crate::errors::ContextMeshError;
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
        #[arg(short, long, default_value = "./src")]
        file: String,
        #[arg(short, long, default_value = "rust")]
        language: String,
    },
    Combine,
    PrintIndex,
}

pub fn run_command(args: Cli) -> Result<(), ContextMeshError> {
    match args.command {
        Commands::Index { file, language } => index::handle_index(&file, &language),
        Commands::Combine => combine::handle_combine(),
        Commands::PrintIndex => print_index::handle_print_index(),
    }
}
