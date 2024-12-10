use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SymbolType {
    Import,
    Function,
    Struct,
    Enum,
    Variable,
    Field,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: String,
    pub line_number: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub dependencies: Vec<String>,
}

impl Symbol {
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.name);
        hasher.update(self.file_path.as_bytes());
        hasher.update(self.line_number.to_string().as_bytes());
        hasher.update(self.start_byte.to_string().as_bytes());
        hasher.update(self.end_byte.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

pub fn calculate_file_hash(file_path: &str) -> Option<String> {
    let content = std::fs::read(file_path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(content);
    Some(format!("{:x}", hasher.finalize()))
}