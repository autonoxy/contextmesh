use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct FileHashManager {
    /// Maps file paths -> their SHA256 content hashes
    file_hashes: HashMap<String, String>,
}

impl FileHashManager {
    pub fn len(&self) -> usize {
        self.file_hashes.len()
    }

    pub fn insert(&mut self, file_path: &str, hash: &str) {
        self.file_hashes
            .insert(file_path.to_string(), hash.to_string());
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.file_hashes.keys()
    }

    pub fn has_changed(&self, file_path: &str, new_hash: &str) -> bool {
        match self.file_hashes.get(file_path) {
            Some(existing) => existing != new_hash,
            None => true,
        }
    }
}
