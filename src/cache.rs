use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    pub file_hashes: HashMap<String, String>, // Maps file paths to their content hashes
    pub symbol_offsets: HashMap<String, Vec<(usize, usize)>>, // Symbol byte offsets
}

impl Cache {
    pub fn new() -> Self {
        Cache::default()
    }

    pub fn load(path: &str) -> Self {
        match std::fs::read(path) {
            Ok(data) => bincode::deserialize(&data).unwrap_or_default(),
            Err(_) => Cache::new(),
        }
    }

    pub fn save(&self, path: &str) {
        if let Ok(data) = bincode::serialize(self) {
            let _ = std::fs::write(path, data);
        }
    }

    pub fn has_changed(&self, file_path: &str, new_hash: &str) -> bool {
        match self.file_hashes.get(file_path) {
            Some(existing_hash) => existing_hash != new_hash,
            None => true,
        }
    }

    pub fn update(
        &mut self,
        file_path: String,
        new_hash: String,
        symbol_offsets: Vec<(usize, usize)>,
    ) {
        self.file_hashes.insert(file_path.clone(), new_hash);
        self.symbol_offsets.insert(file_path, symbol_offsets);
    }
}
