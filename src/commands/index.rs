use std::fs;

use crate::indexer::calculate_file_hash;
use crate::indexer::symbol::{delete_symbol, get_linked_symbols_from_objects, store_symbol};
use crate::indexer::{symbol, Indexer};
use crate::parser::CodeParser;

pub fn handle_index(file: &str, language: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = ".contextmesh";
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir_all(path)?;
        println!("Created directory: {}", path);
    }

    let mut code_parser: CodeParser = initialize_code_parser(language)?;
    let extensions: &[&str] = determine_extensions(language)?;

    let mut indexer: Indexer = load_existing_index();
    let files: Vec<String> = collect_files(file, extensions);

    for file in files {
        process_file(&file, &mut indexer, &mut code_parser)?;
    }

    indexer.save_index()?;

    println!("{:?}", indexer);
    Ok(())
}

fn initialize_code_parser(language: &str) -> Result<CodeParser, Box<dyn std::error::Error>> {
    match language {
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
    match language {
        "rust" => Ok(&["rs"]),
        _ => {
            eprintln!("Unsupported language: {}", language);
            Err(Box::from("Unsupported language."))
        }
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
    let mut files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_files(path.to_str().unwrap(), extensions));
            } else if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap()) {
                    files.push(path.to_str().unwrap().to_string());
                }
            }
        }
    }
    files
}

pub fn process_file(
    file: &str,
    indexer: &mut Indexer,
    code_parser: &mut CodeParser,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing file: '{}'", file);
    let file_hash: String = calculate_file_hash(file).ok_or("File read error")?;

    if indexer.has_changed(file, &file_hash) {
        println!(
            "File '{}' has changed. Performin partial reindexing...",
            file
        );

        // Parse new symbols from the file
        let new_symbols: Vec<symbol::Symbol> = code_parser.parse_file(file);
        if new_symbols.is_empty() {
            eprintln!("No symbols found in '{}'.", file);
        }

        // Process new symbols
        for new_symbol in &new_symbols {
            indexer.add_symbol(new_symbol.clone());
            store_symbol(new_symbol)?;
        }

        indexer.store_file_hash(file, &file_hash);
    } else {
        println!(
            "File '{}' is up-to-date. Adding existing symbols to index.",
            file
        );

        if let Ok(existing_symbols) = Indexer::load_index() {
            for symbol in existing_symbols.get_symbols().values() {
                if symbol.file_path == file {
                    println!("Adding cached symbol: {:?}", symbol);
                    indexer.add_symbol(symbol.clone());
                }
            }
        }
    }

    let stored_symbols = indexer.get_symbols_for_file(file);
    let linked_symbols = get_linked_symbols_from_objects(file)?;

    for linked_symbol in linked_symbols {
        if !stored_symbols
            .iter()
            .any(|s| s.hash() == linked_symbol.hash())
        {
            println!("Removing stale symbol: {:?}", linked_symbol);
            delete_symbol(&linked_symbol)?;
        }
    }

    Ok(())
}
