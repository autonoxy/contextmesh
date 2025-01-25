use log::{error, info, warn};

use crate::errors::ContextMeshError;
use crate::index::Index;
use crate::parser::CodeParser;
use crate::utils::collect_files;

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

fn ensure_index_directory_exists(path: &str) -> Result<(), ContextMeshError> {
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir_all(path)?;
        info!("Created directory: {}", path);
    }
    Ok(())
}

fn load_index() -> Result<Index, ContextMeshError> {
    info!("Loading index...");
    match Index::load_index() {
        Ok(index) => Ok(index),
        Err(e) => {
            warn!("No existing index found (or failed to load): {e}. Initializing new index.");
            Ok(Index::new())
        }
    }
}

fn prepare_parser(
    language: &str,
) -> Result<(&'static [&'static str], CodeParser), ContextMeshError> {
    // Initialize code parser
    let code_parser = match language.to_lowercase().as_str() {
        "rust" => CodeParser::new_rust().map_err(|e| {
            error!(
                "Failed to initialize CodeParser for language '{}': {}",
                language, e
            );
            e
        })?,
        _ => {
            error!("Unsupported language: {}", language);
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
