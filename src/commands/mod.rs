pub mod combine;
pub mod index;
pub mod print_index;
pub mod symbol_refs;

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
    SymbolRefs {
        // The name of the symbol to find references for
        #[arg(short, long)]
        symbol_name: String,
        // How many lines of context around each reference
        #[arg(short, long, default_value = "3")]
        context_lines: usize,
    },
}

pub fn run_command(args: Cli) -> Result<(), ContextMeshError> {
    match args.command {
        Commands::Index { file, language } => index::handle_index(&file, &language),
        Commands::Combine => combine::handle_combine(),
        Commands::PrintIndex => print_index::handle_print_index(),
        Commands::SymbolRefs {
            symbol_name,
            context_lines,
        } => symbol_refs::handle_symbol_refs(&symbol_name, context_lines),
    }
}
