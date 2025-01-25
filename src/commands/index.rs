use log::{info, warn};

use crate::errors::ContextMeshError;
use crate::index::Index;
use crate::indexer::Indexer;
use crate::parser::CodeParser;
use crate::utils::collect_files;

fn load_index() -> Result<Index, ContextMeshError> {
    println!("Loading index...");
    match Index::load_index() {
        Ok(index) => Ok(index),
        Err(e) => {
            warn!("No existing index found (or failed to load): {e}. Initializing new index.");
            Ok(Index::new())
        }
    }
}

pub fn handle_index(dir_or_file: &str, language: &str) -> Result<(), ContextMeshError> {
    ensure_index_directory_exists(".contextmesh")?;
    let mut index = load_index()?;

    // Prepare parser
    let (extensions, mut code_parser) = prepare_parser(language)?;

    // Gather all candidate files (based on extension)
    let files = collect_files(dir_or_file, extensions);

    for file_path in files {
        index.index_file(file_path, &mut code_parser)?;
    }

    index.save_index()?;

    info!("Index updated successfully.");

    Ok(())
}

/*
pub fn handle_index(dir_or_file: &str, language: &str) -> Result<(), ContextMeshError> {
    // Ensure .contextmesh directory
    ensure_index_directory_exists(".contextmesh")?;
    let mut indexer = load_or_create_index()?;

    // Prepare parser
    let (extensions, mut code_parser) = prepare_parser(language)?;

    // Gather all candidate files (based on extension)
    let files = collect_files(dir_or_file, extensions);

    // Process each changed file individually
    for file_path in files {
        indexer.index_file(file_path, &mut code_parser)?;
    }

    // Re-check any unresolved references (forward references, etc.)
    indexer.recheck_unresolved();

    // Save the updated index
    indexer.save_index()?;
    info!("Incremental index updated successfully.");
    println!("Index updated successfully.");

    Ok(())
}
*/

fn ensure_index_directory_exists(path: &str) -> Result<(), ContextMeshError> {
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir_all(path)?;
        info!("Created directory: {}", path);
    }
    Ok(())
}

fn load_or_create_index() -> Result<Indexer, ContextMeshError> {
    println!("Loading existing index...");
    match Indexer::load_index() {
        Ok(existing) => Ok(existing),
        Err(e) => {
            warn!("No existing index found (or failed to load): {e}. Creating a new one.");
            Ok(Indexer::new())
        }
    }
}

fn prepare_parser(
    language: &str,
) -> Result<(&'static [&'static str], CodeParser), ContextMeshError> {
    // Initialize code parser
    let code_parser = match language.to_lowercase().as_str() {
        "rust" => CodeParser::new_rust().map_err(|e| {
            eprintln!(
                "Failed to initialize CodeParser for language '{}': {}",
                language, e
            );
            e
        })?,
        _ => {
            eprintln!("Unsupported language: {}", language);
            return Err(ContextMeshError::UnsupportedLanguage(language.to_string()));
        }
    };

    // Determine extensions
    let extensions = match language.to_lowercase().as_str() {
        "rust" => &["rs"],
        _ => return Err(ContextMeshError::UnsupportedLanguage(language.to_string())),
    };

    Ok((extensions, code_parser))
}
