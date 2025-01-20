pub mod file_hashes;
pub mod symbol;

use crate::errors::ContextMeshError;
use crate::indexer::symbol::Symbol;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

use file_hashes::FileHashManager;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Indexer {
    /// Maps file paths -> their SHA256 content hashes
    file_hashes: FileHashManager,

    /// Maps unique symbol hashes -> their Symbol structures
    symbols: HashMap<String, Symbol>,

    /// Records references that can't be resolved yet (e.g., forward references).
    /// Key = caller symbol hash, Value = list of raw names that don't exist yet.
    unresolved_deps: HashMap<String, Vec<String>>,
}

impl Indexer {
    /// Create a new, empty Indexer
    pub fn new() -> Self {
        Indexer::default()
    }

    // ----------------- Loading / Saving -----------------

    pub fn load_index() -> Result<Self, ContextMeshError> {
        let path = ".contextmesh/index.bin";
        if !std::path::Path::new(path).exists() {
            return Err(ContextMeshError::IndexNotFound(path.to_string()));
        }

        let data = fs::read(path).map_err(ContextMeshError::IoError)?;
        let indexer: Indexer = bincode::deserialize(&data)
            .map_err(|e| ContextMeshError::DeserializationError(e.to_string()))?;

        println!(
            "Loaded index: {} file(s), {} symbol(s).",
            indexer.file_hashes.len(),
            indexer.symbols.len()
        );

        Ok(indexer)
    }

    pub fn save_index(&self) -> Result<(), ContextMeshError> {
        let path = ".contextmesh/index.bin";
        let encoded = bincode::serialize(self)
            .map_err(|e| ContextMeshError::SerializationError(e.to_string()))?;

        fs::write(path, encoded)?;

        println!(
            "Index saved: {} file(s), {} symbol(s), unresolved references: {}.",
            self.file_hashes.len(),
            self.symbols.len(),
            self.unresolved_deps.len()
        );

        Ok(())
    }

    // ----------------- File Hash Logic -----------------

    pub fn has_changed(&self, file_path: &str, new_hash: &str) -> bool {
        self.file_hashes.has_changed(file_path, new_hash)
    }

    pub fn store_file_hash(&mut self, file_path: &str, file_hash: &str) {
        self.file_hashes.insert(file_path, file_hash);
    }

    pub fn get_file_hashes(&self) -> &FileHashManager {
        &self.file_hashes
    }

    // ----------------- Name Map -----------------

    /// Build a map from name -> list of symbol hashes
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

    // ----------------- Symbol Insert / Remove / Link -----------------

    /// Insert or update a Symbol by its hash. Returns old symbol if replaced.
    pub fn add_symbol(&mut self, symbol: Symbol) -> Option<Symbol> {
        let hash = symbol.hash();
        self.symbols.insert(hash, symbol)
    }

    /// Remove a symbol by hash, returning it if it existed.
    pub fn remove_symbol(&mut self, sym_hash: &str) -> Option<Symbol> {
        let removed_sym = self.symbols.remove(sym_hash);

        if removed_sym.is_some() {
            for s in self.symbols.values_mut() {
                s.used_by.remove(sym_hash);
            }
        }

        removed_sym
    }

    /// Retrieve an immutable reference to the entire symbol map
    pub fn get_symbols(&self) -> &HashMap<String, Symbol> {
        &self.symbols
    }

    /// Append `caller_hash` into the `used_by` of `callee_hash`.
    pub fn add_used_by(&mut self, callee_hash: &str, caller_hash: &str) -> bool {
        if let Some(sym) = self.symbols.get_mut(callee_hash) {
            sym.used_by.insert(caller_hash.to_string());
            true
        } else {
            false
        }
    }

    // ----------------- Unresolved Deps -----------------

    /// Record a raw dependency that doesn't yet exist in the index
    pub fn add_unresolved_dep(&mut self, caller_hash: String, missing_name: String) {
        self.unresolved_deps
            .entry(caller_hash)
            .or_default()
            .push(missing_name);
    }

    /// Attempt to recheck the unresolved references. Any that can now be found
    /// (because we've parsed more files) will be resolved; leftover remain unresolved.
    pub fn recheck_unresolved(&mut self) {
        use std::collections::HashMap;

        // We'll move the entire map out of self so we don't hold a mutable borrow of unresolved_deps
        let drained: Vec<(String, Vec<String>)> = self.unresolved_deps.drain().collect();
        let mut still_unresolved = HashMap::new();

        // Build a name map once here. If you expect the name map to change as we fix references,
        // you can rebuild it inside the loop or after each fix. But typically once at the start is enough.
        let global_map = self.build_name_map();

        // For each (caller_symbol_hash, list_of_missing_names)
        for (caller_hash, missing_names) in drained {
            // We remove the caller symbol so we can safely mutate it offline
            if let Some(mut caller_sym) = self.remove_symbol(&caller_hash) {
                let mut leftover = Vec::new();
                // We'll store (dep_hash, caller_hash) pairs we want to link
                let mut used_by_links = Vec::new();

                // For each missing name
                for raw_name in missing_names {
                    if let Some(dep_hashes) = global_map.get(&raw_name) {
                        // We can link them now => but we won't call add_used_by just yet
                        for dep_h in dep_hashes {
                            if dep_h != &caller_hash {
                                // 1) Add a new dependency to caller_sym
                                caller_sym.dependencies.insert(dep_h.clone());
                                // 2) Record that we'll do `add_used_by(dep_h, caller_hash)` later
                                used_by_links.push((dep_h.clone(), caller_hash.clone()));
                            }
                        }
                    } else {
                        // still can't resolve => leftover
                        leftover.push(raw_name);
                    }
                }

                // Reinsert the symbol in the index
                self.add_symbol(caller_sym);

                // Now that caller_sym is no longer borrowed, we can safely call `add_used_by`.
                for (dep_h, caller_h) in used_by_links {
                    self.add_used_by(&dep_h, &caller_h);
                }

                // If leftover is not empty, we put them back in `still_unresolved`
                if !leftover.is_empty() {
                    still_unresolved.insert(caller_hash, leftover);
                }
            } else {
                // If the symbol no longer exists in the index, skip
                // (maybe it was removed or renamed)
            }
        }

        // Update self.unresolved_deps with the ones we still couldn't fix
        self.unresolved_deps = still_unresolved;
    }
}
