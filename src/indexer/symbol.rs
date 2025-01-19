use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// Represents a symbol extracted from the codebase.
///
/// A `Symbol` encapsulates metadata about a particular entity in the code, such as
/// functions, structs, enums, traits, etc. It includes information about the symbol's
/// name, its kind (node kind), location within the source file, and its dependencies
/// on other symbols.
///
/// The `Symbol` struct is serializable and deserializable, allowing it to be easily
/// stored and retrieved from persistent storage formats.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// The name of the symbol (e.g., function name, struct name).
    pub name: String,

    /// The kind of AST node representing the symbol (e.g., `function_item`, `struct_item`).
    pub node_kind: String,

    /// The file path where the symbol is defined.
    pub file_path: String,

    /// The line number in the source file where the symbol is located.
    pub line_number: usize,

    /// The starting byte offset of the symbol in the source file.
    pub start_byte: usize,

    /// The ending byte offset of the symbol in the source file.
    pub end_byte: usize,

    /// A list of hashes representing symbols that this symbol depends on.
    ///
    /// Dependencies indicate relationships where this symbol relies on other symbols,
    /// such as function calls, trait implementations, or struct field types.
    pub dependencies: HashSet<String>,

    /// A list of hashes representing symbols that depend on this symbol.
    ///
    /// The `used_by` field establishes reverse dependencies, showing which symbols
    /// are influenced or utilize this symbol.
    pub used_by: HashSet<String>,
}

impl Symbol {
    /// Calculates a SHA256 hash of the symbol's key attributes.
    ///
    /// This hash uniquely identifies the symbol based on its name, kind, location,
    /// and position within the source file. It can be used to efficiently compare
    /// symbols or track changes across different versions of the codebase.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::symbol::Symbol;
    ///
    /// let symbol = Symbol {
    ///     name: "my_function".to_string(),
    ///     node_kind: "function_item".to_string(),
    ///     file_path: "src/lib.rs".to_string(),
    ///     line_number: 10,
    ///     start_byte: 100,
    ///     end_byte: 150,
    ///     dependencies: vec!["hash_dep1".to_string()],
    ///     used_by: vec!["hash_user1".to_string()],
    /// };
    ///
    /// let symbol_hash = symbol.hash();
    /// println!("Symbol Hash: {}", symbol_hash);
    /// ```
    ///
    /// # Returns
    ///
    /// A `String` containing the hexadecimal representation of the SHA256 hash.
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
