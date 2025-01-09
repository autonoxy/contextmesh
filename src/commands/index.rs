// src/commands/index.rs

use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::errors::ContextMeshError;
use crate::indexer::calculate_file_hash;
use crate::indexer::symbol::Symbol;
use crate::indexer::Indexer;
use crate::parser::CodeParser;

pub fn handle_index(file: &str, language: &str) -> Result<(), ContextMeshError> {
    // Initialize logging (ensure this is set up in your main function)
    // For example, in main.rs:
    // env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // 1) Ensure .contextmesh directory exists
    let index_dir = ".contextmesh";
    if !std::path::Path::new(index_dir).exists() {
        std::fs::create_dir_all(index_dir)?;
        info!("Created directory: {}", index_dir);
    }

    // 2) Load existing index and set up parser
    let mut code_parser: CodeParser = initialize_code_parser(language)?;
    let extensions: &[&str] = determine_extensions(language)?;
    let mut indexer: Indexer = load_existing_index();

    // 3) Collect files that match your extension(s)
    let files: Vec<String> = collect_files(file, extensions);
    let mut new_symbols = Vec::new();

    for file_path in &files {
        let file_hash: String = match calculate_file_hash(file_path) {
            Some(h) => h,
            None => {
                warn!("Could not read/hash file '{}'. Skipping.", file_path);
                continue;
            }
        };

        // Only parse if the file has changed
        if indexer.has_changed(file_path, &file_hash) {
            info!("File '{}' changed. Parsing...", file_path);
            // **Remove the 'language' argument here**
            let (parsed_syms, _parsed_imports) = code_parser.parse_file(file_path)?;

            new_symbols.extend(parsed_syms);
            indexer.store_file_hash(file_path, &file_hash);
            debug!("Parsed and stored symbols from '{}'.", file_path);
        } else {
            debug!("File '{}' is up-to-date. Skipping parse.", file_path);
        }
    }

    let mut merged_symbols: Vec<Symbol> = indexer.get_symbols().values().cloned().collect();
    merged_symbols.extend(new_symbols);
    debug!("Merged symbols: {} total.", merged_symbols.len());

    let mut name_to_hash: HashMap<String, Vec<String>> = HashMap::new();
    for sym in &merged_symbols {
        name_to_hash
            .entry(sym.name.clone())
            .or_default()
            .push(sym.hash());
    }
    debug!(
        "Built name_to_hash map with {} unique names.",
        name_to_hash.len()
    );

    let mut symbol_map: HashMap<String, Symbol> = HashMap::new();
    for sym in merged_symbols {
        let sym_hash = sym.hash();
        symbol_map.insert(sym_hash.clone(), sym);
    }
    debug!(
        "Moved symbols into symbol_map with {} symbols.",
        symbol_map.len()
    );

    // We'll keep track of edges (caller -> callee) in a separate vector
    let mut edges = Vec::new();

    // ----------------------------------------------------------------
    // STEP D) First pass: fix forward dependencies, build edge list
    // ----------------------------------------------------------------
    // Convert each symbol's dependencies (raw names) into hashed references.
    // Also store (caller, callee) in `edges` to fill `used_by` later.
    for (this_hash, sym) in symbol_map.iter_mut() {
        let mut unique_deps = HashSet::new();

        for raw_name in &sym.dependencies {
            if let Some(dep_hashes) = name_to_hash.get(raw_name) {
                for dep_hash in dep_hashes {
                    // Prevent self-dependency
                    if dep_hash != this_hash && unique_deps.insert(dep_hash.clone()) {
                        edges.push((this_hash.clone(), dep_hash.clone()));
                        debug!(
                            "Symbol '{}' (Hash: {}) depends on Symbol Hash: {}",
                            sym.name, this_hash, dep_hash
                        );
                    }
                }
            } else {
                warn!(
                    "Dependency '{}' for Symbol '{}' (Hash: {}) not found in name_map.",
                    raw_name, sym.name, this_hash
                );
            }
        }

        // **Directly mutate the symbol's dependencies without using get_mut**
        sym.dependencies = unique_deps.into_iter().collect();
    }
    info!("First pass completed: Resolved dependencies and built edge list.");

    // ----------------------------------------------------------------
    // STEP E) Second pass: fill in reverse edges (used_by)
    // ----------------------------------------------------------------
    // Iterate through `edges` to populate `used_by` for each callee symbol.
    for (caller_hash, callee_hash) in &edges {
        if let Some(dep_sym) = symbol_map.get_mut(callee_hash) {
            dep_sym.used_by.push(caller_hash.clone());
            debug!(
                "Symbol Hash: {} is used by Symbol Hash: {}",
                callee_hash, caller_hash
            );
        } else {
            warn!(
                "Callee Symbol Hash: {} not found while updating 'used_by'.",
                callee_hash
            );
        }
    }
    info!("Second pass completed: Populated 'used_by' fields.");

    // ----------------------------------------------------------------
    // STEP F) Replace index's symbols with the updated map
    // ----------------------------------------------------------------
    indexer.replace_symbols(symbol_map);
    debug!("Replaced indexer's symbols with updated symbol_map.");

    // Save the updated index
    indexer.save_index()?;
    info!("Index saved successfully.");

    println!("Index updated successfully.");
    Ok(())
}

fn initialize_code_parser(language: &str) -> Result<CodeParser, ContextMeshError> {
    match language.to_lowercase().as_str() {
        "rust" => CodeParser::new_rust().map_err(|e| {
            eprintln!(
                "Failed to initialize CodeParser for language '{}': {}",
                language, e
            );
            e
        }),
        _ => {
            eprintln!("Unsupported language: {}", language);
            Err(ContextMeshError::UnsupportedLanguage(language.to_string()))
        }
    }
}

fn determine_extensions(language: &str) -> Result<&'static [&'static str], ContextMeshError> {
    match language.to_lowercase().as_str() {
        "rust" => Ok(&["rs"]),
        _ => Err(ContextMeshError::UnsupportedLanguage(language.to_string())),
    }
}

fn load_existing_index() -> Indexer {
    println!("Loading existing index...");
    match Indexer::load_index() {
        Ok(existing_indexer) => existing_indexer,
        Err(_) => Indexer::new(),
    }
}

fn collect_files(directory: &str, extensions: &[&str]) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            // Skip hidden dirs, target, etc.
            if file_name.starts_with(".")
                || file_name == "target"
                || file_name == "node_modules"
                || file_name == "tests"
            {
                continue;
            }
            if path.is_dir() {
                files.extend(collect_files(path.to_str().unwrap(), extensions));
            } else if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap()) {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    files
}
