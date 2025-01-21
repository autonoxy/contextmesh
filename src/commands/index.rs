use log::{debug, info, warn};

use crate::errors::ContextMeshError;
use crate::indexer::{file_hashes::calculate_file_hash, Indexer};
use crate::parser::CodeParser;
use crate::utils::collect_files;

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
        let new_hash = match calculate_file_hash(&file_path) {
            Some(h) => h,
            None => {
                warn!("Could not read/hash file '{}'. Skipping.", file_path);
                continue;
            }
        };

        // Only parse if changed
        if indexer.file_hashes.has_changed(&file_path, &new_hash) {
            info!("File '{}' changed. Parsing now...", file_path);

            // Remove old/renamed/deleted symbols from this file
            indexer.remove_deleted_symbols_in_file(&file_path);
            indexer.parse_and_index_file(&file_path, &new_hash, &mut code_parser)?;
        } else {
            debug!("File '{}' is up-to-date. Skipping parse.", file_path);
        }
    }

    // Re-check any unresolved references (forward references, etc.)
    indexer.recheck_unresolved();

    // Save the updated index
    indexer.save_index()?;
    info!("Incremental index updated successfully.");
    println!("Index updated successfully.");

    Ok(())
}

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
