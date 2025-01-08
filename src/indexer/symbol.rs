use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub node_kind: String,
    pub file_path: String,
    pub line_number: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub dependencies: Vec<String>,
    pub used_by: Vec<String>,
}

impl Symbol {
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.name);
        hasher.update(self.node_kind.as_bytes());
        hasher.update(self.file_path.as_bytes());
        hasher.update(self.line_number.to_string().as_bytes());
        hasher.update(self.start_byte.to_string().as_bytes());
        hasher.update(self.end_byte.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
