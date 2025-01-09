use std::fs;

use crate::errors::ContextMeshError;
use crate::indexer::Indexer;

/// Show all symbols that depend on a given symbol (by name)
/// and print context lines around the referencing location.
pub fn handle_symbol_refs(symbol_name: &str, context_lines: usize) -> Result<(), ContextMeshError> {
    let indexer = Indexer::load_index()?;

    // Build name map: symbol name -> list of symbol hashes
    let name_map = indexer.build_name_map();

    // Find symbol hashes for the requested name
    let Some(symbol_hashes) = name_map.get(symbol_name) else {
        println!("No symbol found for name '{}'.", symbol_name);
        return Ok(());
    };

    // For each matching symbol, find all symbols referencing it
    for sym_hash in symbol_hashes {
        let Some(target_sym) = indexer.get_symbols().get(sym_hash) else {
            continue;
        };

        // We want to see who references sym_hash in their dependencies
        let mut referencing_symbols = Vec::new();
        for (_other_hash, other_sym) in indexer.get_symbols() {
            // If dependencies contain `sym_hash`, then other_sym references target_sym
            if other_sym.dependencies.contains(sym_hash) {
                referencing_symbols.push(other_sym.clone());
            }
        }

        println!(
            "Symbol '{}' (hash = {}) is referenced by:",
            target_sym.name, sym_hash
        );

        // Print context for each referencing symbol
        for ref_sym in referencing_symbols {
            println!(" - {} (in file {})", ref_sym.name, ref_sym.file_path);

            // Optional: read lines around ref_sym.line_number
            match fs::read_to_string(&ref_sym.file_path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    // ref_sym.line_number is 1-based, so do minus 1 for indexing
                    let line_idx = ref_sym.line_number.saturating_sub(1);
                    let lower_bound = line_idx.saturating_sub(context_lines);
                    let upper_bound = (line_idx + context_lines + 1).min(lines.len());

                    for i in lower_bound..upper_bound {
                        println!("{:4} | {}", i + 1, lines[i]);
                    }
                    println!("---");
                }
                Err(e) => {
                    eprintln!(
                        "Failed to read file '{}': {}. Skipping context lines.",
                        ref_sym.file_path, e
                    );
                }
            }
        }
    }

    Ok(())
}
