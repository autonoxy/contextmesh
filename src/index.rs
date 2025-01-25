use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::mem::take;
use std::path::Path;
use std::{
    collections::{HashMap, HashSet},
    fs,
};

use crate::parser::CodeParser;
use crate::utils::calculate_file_hash;
use crate::{errors::ContextMeshError, symbol::Symbol};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Index {
    /// Maps file paths -> their SHA256 content hashes
    pub file_hashes: HashMap<String, String>,

    /// Maps unique symbol hashes -> their Symbol structure
    pub symbols: HashMap<String, Symbol>,

    /// Records references that can't be resolved yet (e.g., forward references).
    /// Key = caller hash symbol, Value = list of raw names that don't exist yet.
    unresolved_dependencies: HashMap<String, Vec<String>>,

    /// Live name map for quick name->symbol lookups
    #[serde(skip)]
    name_map: HashMap<String, Vec<String>>,
}

impl Index {
    const INDEX_FILE_PATH: &'static str = ".contextmesh/index.bin";

    pub fn new() -> Self {
        Index::default()
    }

    pub fn load_index() -> Result<Self, ContextMeshError> {
        if !Path::new(Self::INDEX_FILE_PATH).exists() {
            return Err(ContextMeshError::IndexNotFound(
                Self::INDEX_FILE_PATH.to_string(),
            ));
        }

        let data = fs::read(Self::INDEX_FILE_PATH).map_err(ContextMeshError::IoError)?;
        let mut index: Index = bincode::deserialize(&data)
            .map_err(|e| ContextMeshError::DeserializationError(e.to_string()))?;

        index.build_name_map();

        info!(
            "Loaded index: {} file(s), {} symbol(s).",
            index.file_hashes.len(),
            index.symbols.len()
        );

        Ok(index)
    }

    pub fn save_index(&self) -> Result<(), ContextMeshError> {
        let encoded = bincode::serialize(self)
            .map_err(|e| ContextMeshError::SerializationError(e.to_string()))?;
        fs::write(Self::INDEX_FILE_PATH, encoded)?;

        info!(
            "Index saved: {} file(s), {} symbol(s), unresolved references: {}.",
            self.file_hashes.len(),
            self.symbols.len(),
            self.unresolved_dependencies.len()
        );

        Ok(())
    }

    pub fn index_file(
        &mut self,
        file_path: String,
        code_parser: &mut CodeParser,
    ) -> Result<(), ContextMeshError> {
        let new_hash = match calculate_file_hash(&file_path) {
            Some(h) => h,
            None => {
                warn!("Could not read/hash file '{}'. Skipping.", file_path);
                return Ok(());
            }
        };

        let file_has_changed = self
            .file_hashes
            .get(&file_path)
            .map_or(true, |existing| existing != &new_hash);

        if file_has_changed {
            info!("File '{}' changed. Parsing now...", file_path);

            // Parse all symbols from changed file
            let (parsed_syms, _imports) = code_parser.parse_file(&file_path)?;
            debug!("Parsed {} symbols from '{}'.", parsed_syms.len(), file_path);

            // Remove old symbols associated with the file using retain
            let mut old_hashes = Vec::new();
            for (hash, sym) in &self.symbols {
                if sym.file_path == file_path {
                    old_hashes.push(hash.clone());
                }
            }
            for h in old_hashes {
                self.remove_symbol(&h);
            }

            // Insert new symbols
            for sym in &parsed_syms {
                self.add_symbol(sym.clone());
            }

            // Resolve dependencies right away, linking to local or global symbols
            self.resolve_new_symbols_dependencies(&parsed_syms, &file_path);

            // Update the file hashes
            self.file_hashes.insert(file_path.clone(), new_hash);
            debug!("Finished incremental update for '{}'.", &file_path);
        } else {
            debug!("File '{}' is up-to-date. Skipping parse.", file_path);
        }

        Ok(())
    }

    fn resolve_new_symbols_dependencies(&mut self, new_symbols: &[Symbol], file_path: &str) {
        // A temporary structure to batch updates for `used_by` dependencies
        let mut used_by_updates: HashMap<String, HashSet<String>> = HashMap::new();

        for sym in new_symbols {
            let this_hash = sym.hash();

            if let Some(sym_mut) = self.symbols.get_mut(&this_hash) {
                // Extract and clear the current dependencies
                let old_deps = take(&mut sym_mut.dependencies);
                let mut new_dep_hashes = HashSet::new();

                for raw_name in old_deps {
                    // Collect unique candidates from local and global name maps
                    let mut candidates = self.name_map.get(&raw_name).cloned().unwrap_or_default();

                    // Remove self-dependency
                    candidates.retain(|dep_hash| dep_hash != &this_hash);

                    if candidates.is_empty() {
                        warn!(
                            "Dependency '{}' not found for symbol '{}'. (File: {})",
                            raw_name, sym_mut.name, file_path
                        );
                        // Add to unresolved dependencies
                        self.unresolved_dependencies
                            .entry(this_hash.clone())
                            .or_default()
                            .push(raw_name);
                    } else {
                        // Add all candidates to new_dep_hashes and prepare `used_by` updates
                        new_dep_hashes.extend(candidates.iter().cloned());
                        for dep_hash in candidates {
                            used_by_updates
                                .entry(dep_hash.clone())
                                .or_default()
                                .insert(this_hash.clone());
                        }
                    }
                }

                // Update the symbol's dependencies with resolved hashes
                sym_mut.dependencies = new_dep_hashes.into_iter().collect();
            }
        }

        // Apply the `used_by` updates in a single pass
        for (dep_hash, used_by_set) in used_by_updates {
            if let Some(dep_sym) = self.symbols.get_mut(&dep_hash) {
                dep_sym.used_by.extend(used_by_set);
            }
        }
    }

    fn build_name_map(&mut self) {
        self.name_map.clear();
        for (hash, sym) in &self.symbols {
            self.name_map
                .entry(sym.name.clone())
                .or_default()
                .push(hash.clone());
        }
    }

    fn remove_hash_from_name_map(&mut self, name: &str, sym_hash: &str) {
        if let Some(hashes) = self.name_map.get_mut(name) {
            hashes.retain(|h| h != sym_hash);
            if hashes.is_empty() {
                self.name_map.remove(name);
            }
        }
    }

    fn add_symbol(&mut self, sym: Symbol) {
        let hash = sym.hash();

        if let Some(old_sym) = self.symbols.insert(hash.clone(), sym.clone()) {
            self.remove_hash_from_name_map(&old_sym.name, &hash);
        }

        self.name_map
            .entry(sym.name.clone())
            .or_default()
            .push(hash);
    }

    fn remove_symbol(&mut self, sym_hash: &str) -> Option<Symbol> {
        if let Some(removed_sym) = self.symbols.remove(sym_hash) {
            self.remove_hash_from_name_map(&removed_sym.name, sym_hash);
            Some(removed_sym)
        } else {
            None
        }
    }
}
