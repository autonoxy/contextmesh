use std::fs;

use crate::indexer::{symbol, Indexer};
use crate::parser::CodeParser;
use crate::cache::Cache;

pub fn handle_index(
    file: &str,
    language: &str,
    cache: &mut Cache,
) -> Result<(), Box<dyn std::error::Error>> {
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
        process_file(&file, &mut indexer, &mut code_parser, cache)?;
    }

    save_index(&indexer)?;
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

fn determine_extensions(language: &str) -> Result<&'static [&'static str], Box<dyn std::error::Error>> {
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
        Err(_) => Indexer::new()
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

fn process_file(
    file: &str,
    indexer: &mut Indexer,
    code_parser: &mut CodeParser,
    cache: &mut Cache,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_hash: String = symbol::calculate_file_hash(file).ok_or("File read error")?;

    if cache.has_changed(file, &file_hash) {
        println!(
            "File '{}' has changed. Performin partial reindexing...",
            file
        );

        let symbols: Vec<symbol::Symbol> = code_parser.parse_file(file);
        if symbols.is_empty() {
            eprintln!("No symbols found in '{}'.", file);
        }

        for symbol in &symbols {
            let symbol_hash = symbol.hash();
            if let Some(existing_symbol) = Indexer::load_symbol(&symbol_hash) {
                if existing_symbol != *symbol {
                    println!("Updating Symbol: {:?}", symbol);
                    indexer.add_symbol(symbol.clone());
                    indexer.store_symbol(symbol)?;
                }
            } else {
                println!("Storing New Symbol: {:?}", symbol);
                indexer.add_symbol(symbol.clone());
                indexer.store_symbol(symbol)?;
            }
        }

        cache.update(
            file.to_string(),
            file_hash,
            symbols.iter().map(|s| (s.start_byte, s.end_byte)).collect()
        );
        cache.save(".contextmesh/cache.bin");
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

    Ok(())
}

fn save_index(indexer: &Indexer) -> Result<(), Box<dyn std::error::Error>> {
    println!("Saving merged index...");
    indexer.save_index(".contextmesh/index.bin")?;
    Ok(())
}