pub mod symbol;

use crate::errors::ContextMeshError;
use crate::indexer::symbol::Symbol;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs};

/// `Indexer` is responsible for managing the indexing of source files,
/// tracking file hashes, and maintaining a collection of symbols extracted
/// from the codebase.
///
/// It provides functionalities to load and save the index, check for file changes,
/// and build mappings of symbol names to their unique identifiers.
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Indexer {
    /// Maps file paths to their corresponding SHA256 content hashes.
    ///
    /// This allows the indexer to determine if a file has changed since the last indexing,
    /// facilitating efficient updates by avoiding re-indexing unchanged files.
    file_hashes: HashMap<String, String>, // Maps file paths to their content hashes

    /// Maps unique symbol hashes to their corresponding `Symbol` structures.
    ///
    /// Each `Symbol` contains metadata about a particular symbol in the codebase,
    /// such as its name, kind, location, and dependencies.
    symbols: HashMap<String, Symbol>,
}

impl Indexer {
    /// Creates a new, empty `Indexer` instance.
    ///
    /// This method initializes the `Indexer` with default values, setting up empty
    /// hash maps for `file_hashes` and `symbols`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let indexer = Indexer::new();
    /// assert!(indexer.file_hashes.is_empty());
    /// assert!(indexer.symbols.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// A new instance of `Indexer` with empty `file_hashes` and `symbols`.
    pub fn new() -> Self {
        Indexer::default()
    }

    /// Loads the index from a binary file, deserializing its contents into an `Indexer` instance.
    ///
    /// This method attempts to read the index from the specified path. If the index file
    /// does not exist, it returns an empty `Indexer`. Otherwise, it deserializes the
    /// file's contents into the `Indexer` structure.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::IoError` if the file cannot be read.
    /// Returns `ContextMeshError::DeserializationError` if deserialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::errors::ContextMeshError;
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let indexer = Indexer::load_index().expect("Failed to load index");
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` containing the loaded `Indexer` on success,
    /// or a `ContextMeshError` on failure.
    pub fn load_index() -> Result<Self, ContextMeshError> {
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

        let data = fs::read(path).map_err(|e| ContextMeshError::IoError(e))?;
        let indexer: Indexer = bincode::deserialize(&data)
            .map_err(|e| ContextMeshError::DeserializationError(e.to_string()))?;

        println!(
            "Loaded index: {} file(s), {} symbol(s).",
            indexer.file_hashes.len(),
            indexer.symbols.len(),
        );

        Ok(indexer)
    }

    /// Saves the current index to a binary file by serializing its contents.
    ///
    /// This method serializes the `Indexer` instance using `bincode` and writes the
    /// serialized data to the specified path. It provides feedback on the number
    /// of files and symbols saved.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::SerializationError` if serialization fails.
    /// Returns `ContextMeshError::IoError` if the file cannot be written.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::errors::ContextMeshError;
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let indexer = Indexer::new();
    /// indexer.save_index().expect("Failed to save index");
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the index was successfully saved,
    /// or a `ContextMeshError` if an error occurred.
    pub fn save_index(&self) -> Result<(), ContextMeshError> {
        let path = ".contextmesh/index.bin";
        let encoded = bincode::serialize(self)
            .map_err(|e| ContextMeshError::SerializationError(e.to_string()))?;

        fs::write(path, encoded)?;

        println!(
            "Index saved: {} file(s), {} symbol(s).",
            self.file_hashes.len(),
            self.symbols.len(),
        );

        Ok(())
    }

    /// Retrieves a reference to the symbols stored in the indexer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let indexer = Indexer::new();
    /// let symbols = indexer.get_symbols();
    /// assert!(symbols.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// A reference to the `HashMap` containing symbol hashes mapped to `Symbol` instances.
    pub fn get_symbols(&self) -> &HashMap<String, Symbol> {
        &self.symbols
    }

    /// Replaces the current set of symbols with a new collection.
    ///
    /// This method allows for updating the indexer's symbol table by providing a new
    /// `HashMap` of symbols. Existing symbols will be overwritten by the new ones.
    ///
    /// # Arguments
    ///
    /// * `new_symbols` - A `HashMap` containing the new symbols to replace the existing ones.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::{Indexer, Symbol};
    /// use std::collections::HashMap;
    ///
    /// let mut indexer = Indexer::new();
    /// let mut new_symbols = HashMap::new();
    /// new_symbols.insert("hash1".to_string(), Symbol::default());
    /// indexer.replace_symbols(new_symbols);
    /// assert_eq!(indexer.get_symbols().len(), 1);
    /// ```
    pub fn replace_symbols(&mut self, new_symbols: HashMap<String, Symbol>) {
        self.symbols = new_symbols;
    }

    /// Checks whether a file has changed by comparing its current hash with the stored hash.
    ///
    /// This method determines if a file needs to be re-indexed by verifying if its content
    /// hash has changed since the last indexing. If the file is new or its hash differs
    /// from the stored hash, the method returns `true`.
    ///
    /// # Arguments
    ///
    /// * `file_path` - A string slice representing the path to the file.
    /// * `new_hash` - A string slice containing the newly calculated hash of the file's content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let indexer = Indexer::new();
    /// let has_changed = indexer.has_changed("src/main.rs", "newhashvalue");
    /// assert!(has_changed);
    /// ```
    ///
    /// # Returns
    ///
    /// `true` if the file is new or its hash has changed; `false` otherwise.
    pub fn has_changed(&self, file_path: &str, new_hash: &str) -> bool {
        match self.file_hashes.get(file_path) {
            Some(existing_hash) => existing_hash != new_hash,
            None => true,
        }
    }

    /// Stores or updates the hash of a file in the indexer's hash map.
    ///
    /// This method records the hash of a file, allowing the indexer to track changes
    /// and determine if re-indexing is necessary in the future.
    ///
    /// # Arguments
    ///
    /// * `file` - A string slice representing the path to the file.
    /// * `file_hash` - A string slice containing the hash of the file's content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let mut indexer = Indexer::new();
    /// indexer.store_file_hash("src/main.rs", "hashvalue123");
    /// assert_eq!(indexer.get_file_hashes().get("src/main.rs"), Some(&"hashvalue123".to_string()));
    /// ```
    pub fn store_file_hash(&mut self, file: &str, file_hash: &str) {
        self.file_hashes
            .insert(file.to_string(), file_hash.to_string());
    }

    /// Retrieves a reference to the file hashes stored in the indexer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::Indexer;
    ///
    /// let indexer = Indexer::new();
    /// let file_hashes = indexer.get_file_hashes();
    /// assert!(file_hashes.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// A reference to the `HashMap` containing file paths mapped to their content hashes.
    pub fn get_file_hashes(&self) -> &HashMap<String, String> {
        &self.file_hashes
    }

    /// Builds a mapping from symbol names to their corresponding hashes.
    ///
    /// This method creates a `HashMap` where each key is a symbol's name, and the value
    /// is a vector of hashes representing that symbol. This allows for efficient lookup
    /// of symbols by name, accommodating cases where multiple symbols share the same name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::indexer::symbol::{Indexer, Symbol};
    /// use std::collections::HashMap;
    ///
    /// let mut indexer = Indexer::new();
    /// let mut symbols = HashMap::new();
    /// symbols.insert("hash1".to_string(), Symbol::new("foo"));
    /// symbols.insert("hash2".to_string(), Symbol::new("foo"));
    /// indexer.replace_symbols(symbols);
    ///
    /// let name_map = indexer.build_name_map();
    /// assert_eq!(name_map.get("foo").unwrap().len(), 2);
    /// ```
    ///
    /// # Returns
    ///
    /// A `HashMap` mapping symbol names (`String`) to a vector of their corresponding hashes (`Vec<String>`).
    pub fn build_name_map(&self) -> HashMap<String, Vec<String>> {
        let mut name_map = HashMap::new();
        for (hash, sym) in &self.symbols {
            name_map
                .entry(sym.name.clone())
                .or_insert_with(Vec::new)
                .push(hash.clone());
        }
        name_map
    }
}

/// Calculates the SHA256 hash of a file's content.
///
/// This function reads the specified file, computes its SHA256 hash, and returns
/// the hash as a hexadecimal `String`. If the file cannot be read, it returns `None`.
///
/// # Arguments
///
/// * `file_path` - A string slice representing the path to the file.
///
/// # Examples
///
/// ```rust
/// use crate::indexer::symbol::calculate_file_hash;
///
/// let hash = calculate_file_hash("src/main.rs").expect("Failed to calculate file hash");
/// println!("File hash: {}", hash);
/// ```
///
/// # Returns
///
/// An `Option<String>` containing the hexadecimal SHA256 hash of the file's content if successful,
/// or `None` if the file could not be read.
pub fn calculate_file_hash(file_path: &str) -> Option<String> {
    let content = std::fs::read(file_path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(content);
    Some(format!("{:x}", hasher.finalize()))
}
