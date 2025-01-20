use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;

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

pub fn calculate_file_hash(file_path: &str) -> Option<String> {
    let content = fs::read(file_path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(content);
    Some(format!("{:x}", hasher.finalize()))
}
