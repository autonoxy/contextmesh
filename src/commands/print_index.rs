use crate::errors::ContextMeshError;
use crate::index::Index;
use arboard::Clipboard;

pub fn handle_print_index() -> Result<(), ContextMeshError> {
    println!("Loading index...");
    let mut combined_content = String::new();

    let indexer = Index::load_index().map_err(|e| {
        eprintln!("Failed to load index: {}", e);
        e
    })?;

    println!("Indexed symbols:");
    for (hash, symbol) in indexer.symbols {
        let s = format!("Hash: {}, Symbol: {:?}\n", hash, symbol);
        combined_content.push_str(&format!("Hash: {}, Symbol: {:?}\n", hash, symbol));
        println!("{}", s);
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

    Ok(())
}
