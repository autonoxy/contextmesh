use crate::errors::ContextMeshError;
use crate::indexer::Indexer;

pub fn handle_print_index() -> Result<(), ContextMeshError> {
    println!("Loading index...");

    let indexer = Indexer::load_index().map_err(|e| {
        eprintln!("Failed to load index: {}", e);
        e
    })?;

    println!("Indexed symbols:");
    for (hash, symbol) in indexer.get_symbols() {
        println!("Hash: {}, Symbol: {:?}", hash, symbol);
    }

    Ok(())
}
