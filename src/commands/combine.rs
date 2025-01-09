use crate::errors::ContextMeshError;
use crate::indexer::Indexer;
use arboard::Clipboard;
use std::fs;

pub fn handle_combine() -> Result<(), ContextMeshError> {
    let indexer = Indexer::load_index()?;
    let mut combined_content = String::new();

    for file_path in indexer.get_file_hashes().keys() {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                combined_content.push_str(&format!("# {}\n\n{}\n\n", file_path, content));
            }
            Err(e) => {
                eprintln!("Failed to read file '{}': {}. Skipping.", file_path, e);
                // Optionally, you could choose to return an error instead of continuing
            }
        }
    }

    if !combined_content.is_empty() {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                clipboard
                    .set_text(combined_content.clone())
                    .map_err(|e| ContextMeshError::ClipboardError(e.to_string()))?;
                println!("Combined content copied to clipboard.");
            }
            Err(e) => {
                eprintln!("Failed to initialize clipboard: {}.", e);
                return Err(ContextMeshError::ClipboardError(e.to_string()));
            }
        }
    } else {
        println!("No files found to combine.");
    }

    println!("\nCombined Content:\n{}", combined_content);
    Ok(())
}
