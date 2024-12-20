use crate::indexer::{calculate_file_hash, Indexer};

pub fn handle_check(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let indexer: Indexer = load_existing_index();
    let file_hash = calculate_file_hash(file).ok_or("File read error")?;

    if indexer.has_changed(file, &file_hash) {
        println!("File '{}' has changes.", file);
    } else {
        println!("File '{}' is up to date.", file);
    }
    Ok(())
}

fn load_existing_index() -> Indexer {
    println!("Loading existing index...");
    match Indexer::load_index() {
        Ok(existing_indexer) => existing_indexer,
        Err(_) => Indexer::new(),
    }
}
