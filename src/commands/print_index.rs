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
    for symbol in indexer.get_symbols().values() {
        println!("Symbol: {:?}", symbol);
    }

    Ok(())
}