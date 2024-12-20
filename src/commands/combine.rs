use crate::indexer::Indexer;
use arboard::Clipboard;
use std::fs;

pub fn handle_combine() -> Result<(), Box<dyn std::error::Error>> {
    let indexer = Indexer::load_index()?;
    let mut combined_content = String::new();

    for file_path in indexer.get_file_hashes().keys() {
        if let Ok(content) = fs::read_to_string(file_path) {
            combined_content.push_str(&format!("# {}\n\n{}\n\n", file_path, content));
        } else {
            eprintln!("Failed to read file '{}'. Skipping.", file_path);
        }
    }

    if !combined_content.is_empty() {
        Clipboard::new()
            .ok()
            .and_then(|mut clipboard| clipboard.set_text(combined_content.clone()).ok());
        println!("Combined content copied to clipboard.");
    } else {
        println!("No files found to combine.");
    }

    println!("\nCombined Content:\n{}", combined_content);
    Ok(())
}

