use crate::{cache::Cache, indexer::calculate_file_hash};

pub fn handle_check(file: &str, cache: &mut Cache) -> Result<(), Box<dyn std::error::Error>> {
    let file_hash = calculate_file_hash(file).ok_or("File read error")?;

    if cache.has_changed(file, &file_hash) {
        println!("File '{}' has changes.", file);
    } else {
        println!("File '{}' is up to date.", file);
    }
    Ok(())
}

