use crate::indexer::Indexer;

pub fn handle_print_index() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading index...");

    let indexer = match Indexer::load_index() {
        Ok(indexer) => indexer,
        Err(e) => {
            eprintln!("Failed to load index: {}", e);
            return Err(Box::from("Failed to load index"));
        }
    };

    println!("Indexed symbols:");
    for (hash, symbol) in indexer.get_symbols() {
        println!("Hash: {}, Symbol: {:?}", hash, symbol);
    }

    Ok(())
}

