pub mod symbol;

use crate::indexer::symbol::Symbol;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Error, ErrorKind},
};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Indexer {
    file_hashes: HashMap<String, String>, // Maps file paths to their content hashes
    symbols: HashMap<String, Symbol>,
}

impl Indexer {
    pub fn new() -> Self {
        Indexer::default()
    }

    pub fn load_index() -> std::io::Result<Self> {
        let path = ".contextmesh/index.bin";

        if !std::path::Path::new(path).exists() {
            eprintln!(
                "Index file '{}' does not exist. Returning empty index.",
                path
            );
            return Ok(Indexer {
                file_hashes: HashMap::new(),
                symbols: HashMap::new(),
            });
        }

        let data = fs::read(path).map_err(|e| {
            eprintln!("Failed to read index file '{}': {}", path, e);
            Error::new(ErrorKind::Other, "Failed to read index file.")
        })?;

        let indexer: Indexer = bincode::deserialize(&data).map_err(|e| {
            eprintln!("Failed to deserialize index file: '{}': {}", path, e);
            Error::new(ErrorKind::Other, "Deserialization failed.")
        })?;

        println!(
            "Loaded index: {} file, {} symbols.",
            indexer.file_hashes.len(),
            indexer.symbols.len(),
        );

        Ok(indexer)
    }

    pub fn save_index(&self) -> std::io::Result<()> {
        let path = ".contextmesh/index.bin";
        let encoded = bincode::serialize(self)
            .map_err(|e| Error::new(ErrorKind::Other, format!("Serialization failed: {}", e)))?;

        fs::write(path, encoded)?;

        println!(
            "Index saved: {} files, {} symbols.",
            self.file_hashes.len(),
            self.symbols.len(),
        );

        Ok(())
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    /*
    pub fn load_symbol(hash: &str) -> Option<Symbol> {
        let dir = format!(".contextmesh/objects/{}/", &hash[0..2]);
        let file_path = format!("{}.bin", &hash[2..]);
        let full_path = Path::new(&dir).join(file_path);

        let data = fs::read(full_path).ok()?;
        bincode::deserialize(&data).ok()
    }
    */

    pub fn get_symbols(&self) -> &HashMap<String, Symbol> {
        &self.symbols
    }

    pub fn get_symbols_for_file(&self, file: &str) -> Vec<symbol::Symbol> {
        self.get_symbols()
            .values()
            .filter(|symbol| symbol.file_path == file)
            .cloned()
            .collect()
    }

    pub fn has_changed(&self, file_path: &str, new_hash: &str) -> bool {
        match self.file_hashes.get(file_path) {
            Some(existing_hash) => existing_hash != new_hash,
            None => true,
        }
    }

    pub fn store_file_hash(&mut self, file: &str, file_hash: &str) {
        self.file_hashes
            .insert(file.to_string(), file_hash.to_string());
    }
}

pub fn calculate_file_hash(file_path: &str) -> Option<String> {
    let content = std::fs::read(file_path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(content);
    Some(format!("{:x}", hasher.finalize()))
}
