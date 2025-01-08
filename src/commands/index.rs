// src/commands/index.rs

use std::collections::HashMap;
use std::fs;

use crate::indexer::calculate_file_hash;
use crate::indexer::symbol::Symbol;
use crate::indexer::Indexer;
use crate::parser::CodeParser;

pub fn handle_index(file: &str, language: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1) Ensure .contextmesh directory
    let path = ".contextmesh";
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir_all(path)?;
        println!("Created directory: {}", path);
    }

    // 2) Load existing index, set up parser
    let mut code_parser: CodeParser = initialize_code_parser(language)?;
    let extensions: &[&str] = determine_extensions(language)?;
    let mut indexer: Indexer = load_existing_index();

    // 3) Collect files that match your extension(s)
    let files: Vec<String> = collect_files(file, extensions);
    let mut new_symbols = Vec::new();

    for file_path in files {
        let file_hash: String = match calculate_file_hash(&file_path) {
            Some(h) => h,
            None => {
                eprintln!("Could not read/hash file '{}'", file_path);
                continue;
            }
        };

        // Only parse if the file has changed
        if indexer.has_changed(&file_path, &file_hash) {
            println!("File '{}' changed. Parsing...", file_path);
            // **Remove the 'language' argument here**
            let (parsed_syms, _parsed_imports) = code_parser.parse_file(&file_path);

            new_symbols.extend(parsed_syms);
            indexer.store_file_hash(&file_path, &file_hash);
        } else {
            println!("File '{}' is up-to-date. Skipping parse.", file_path);
        }
    }

    // ----------------------------------------------------------------
    // STEP A) Merge old + new symbols into one list
    // ----------------------------------------------------------------
    let mut merged_symbols: Vec<Symbol> = indexer.get_symbols().values().cloned().collect();
    merged_symbols.extend(new_symbols);

    // ----------------------------------------------------------------
    // STEP B) Build name->hash map for ALL symbols
    //         (so we can resolve references from "raw name" -> symbol hash)
    // ----------------------------------------------------------------
    let mut name_to_hash = HashMap::new();
    for sym in &merged_symbols {
        name_to_hash.insert(sym.name.clone(), sym.hash());
    }

    // ----------------------------------------------------------------
    // STEP C) Move symbols into a HashMap<hash, Symbol>
    //         We'll do the reference resolution in two passes to avoid
    //         overlapping mutable borrows.
    // ----------------------------------------------------------------
    let mut symbol_map: HashMap<String, Symbol> = HashMap::new();
    for sym in merged_symbols {
        let sym_hash = sym.hash();
        symbol_map.insert(sym_hash, sym);
    }

    // We'll keep track of edges (caller -> callee) in a separate vector
    let mut edges = Vec::new();

    // ----------------------------------------------------------------
    // STEP D) First pass: fix forward dependencies, build edge list
    // ----------------------------------------------------------------
    // We convert each symbol's dependencies (raw names) into hashed references.
    // Also store (caller, callee) in `edges` so we can fill used_by later.
    let symbol_hashes: Vec<String> = symbol_map.keys().cloned().collect();

    for this_hash in &symbol_hashes {
        if let Some(sym) = symbol_map.get_mut(this_hash) {
            let mut resolved = Vec::new();

            for raw_name in &sym.dependencies {
                if let Some(dep_hash) = name_to_hash.get(raw_name) {
                    resolved.push(dep_hash.clone());
                    // Temporarily record (caller, callee)
                    edges.push((this_hash.clone(), dep_hash.clone()));
                }
            }

            // Overwrite the old dependencies with the hashed versions
            sym.dependencies = resolved;
        }
    }

    // ----------------------------------------------------------------
    // STEP E) Second pass: fill in reverse edges (used_by)
    // ----------------------------------------------------------------
    // We do this in a separate loop, so we don't borrow the same symbol_map
    // mutably twice at once.
    for (caller_hash, callee_hash) in edges {
        if let Some(dep_sym) = symbol_map.get_mut(&callee_hash) {
            dep_sym.used_by.push(caller_hash);
        }
    }

    // ----------------------------------------------------------------
    // STEP F) Replace index's symbols with the updated map
    // ----------------------------------------------------------------
    indexer.replace_symbols(symbol_map);

    // Save index
    indexer.save_index()?;

    println!("Index updated successfully.");
    Ok(())
}

fn initialize_code_parser(language: &str) -> Result<CodeParser, Box<dyn std::error::Error>> {
    match language.to_lowercase().as_str() {
        "rust" => Ok(CodeParser::new_rust()),
        _ => {
            eprintln!("Unsupported language: {}", language);
            Err(Box::from("Unsupported language."))
        }
    }
}

fn determine_extensions(
    language: &str,
) -> Result<&'static [&'static str], Box<dyn std::error::Error>> {
    match language.to_lowercase().as_str() {
        "rust" => Ok(&["rs"]),
        "python" => Ok(&["py"]),
        "kotlin" => Ok(&["kt"]),
        _ => Err("Unsupported language.".into()),
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
