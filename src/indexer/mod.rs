mod dependecy_resolver;
mod file_hashes;
mod symbol_store;

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::mem::take;
use std::{
    collections::{HashMap, HashSet},
    fs,
};

use crate::errors::ContextMeshError;
use crate::parser::CodeParser;
use crate::symbol::Symbol;
use crate::utils::calculate_file_hash;

use self::dependecy_resolver::DependencyResolver;
use self::file_hashes::FileHashManager;
use self::symbol_store::SymbolStore;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Indexer {
    /// Maps file paths -> their SHA256 content hashes
    file_hashes: FileHashManager,

    /// Maps unique symbol hashes -> their Symbol structures
    symbol_store: SymbolStore,

    /// Records references that can't be resolved yet (e.g., forward references).
    /// Key = caller symbol hash, Value = list of raw names that don't exist yet.
    dependency_resolver: DependencyResolver,
}

impl Indexer {
    pub fn new() -> Self {
        Indexer::default()
    }

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
            indexer.symbol_store.len()
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
            self.symbol_store.len(),
            self.dependency_resolver.len()
        );

        Ok(())
    }

    pub fn get_indexed_files(&self) -> impl Iterator<Item = &String> {
        self.file_hashes.keys()
    }

    pub fn get_symbols(&self) -> &HashMap<String, Symbol> {
        self.symbol_store.get_symbols()
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

        // Only parse if changed
        if self.file_hashes.has_changed(&file_path, &new_hash) {
            info!("File '{}' changed. Parsing now...", file_path);

            // Remove old/renamed/deleted symbols from this file
            self.remove_deleted_symbols_in_file(&file_path);
            self.parse_and_index_file(&file_path, &new_hash, code_parser)?;
        } else {
            debug!("File '{}' is up-to-date. Skipping parse.", file_path);
        }

        Ok(())
    }

    fn parse_and_index_file(
        &mut self,
        file_path: &str,
        new_hash: &str,
        code_parser: &mut CodeParser,
    ) -> Result<(), ContextMeshError> {
        // Parse the file => new symbols
        let new_symbols = self.parse_file_symbols(file_path, code_parser)?;

        // Build local name->hash for references *within* this file
        let mut global_name_map = self.symbol_store.build_name_map();
        let local_name_map = self.build_local_name_map(&new_symbols);

        // Insert new symbols into the index & update global name map
        self.insert_new_symbols(&new_symbols, &mut global_name_map);

        // Resolve dependencies right away, linking to local or global symbols
        self.resolve_new_symbols_dependencies(
            &new_symbols,
            &local_name_map,
            &global_name_map,
            file_path,
        );

        // Mark the file hash so we know it's up to date next run
        self.file_hashes.insert(file_path, new_hash);

        debug!("Finished incremental update for '{}'.", file_path);
        Ok(())
    }

    fn remove_deleted_symbols_in_file(&mut self, file_path: &str) {
        let mut global_name_map = self.symbol_store.build_name_map();

        // Gather all existing symbols from this file
        let old_symbols: Vec<(String, String)> = self
            .symbol_store
            .get_symbols()
            .iter()
            .filter_map(|(hash, sym)| {
                if sym.file_path == file_path {
                    Some((hash.clone(), sym.name.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Remove them from index + global map
        for (old_hash, old_name) in old_symbols {
            self.symbol_store.remove_symbol(&old_hash);

            if let Some(vec_of_hashes) = global_name_map.get_mut(&old_name) {
                vec_of_hashes.retain(|h| h != &old_hash);
                if vec_of_hashes.is_empty() {
                    global_name_map.remove(&old_name);
                }
            }
        }
    }

    fn parse_file_symbols(
        &self,
        file_path: &str,
        code_parser: &mut CodeParser,
    ) -> Result<Vec<Symbol>, ContextMeshError> {
        let (parsed_syms, _imports) = code_parser.parse_file(file_path)?;
        debug!("Parsed {} symbols from '{}'.", parsed_syms.len(), file_path);
        Ok(parsed_syms)
    }

    fn build_local_name_map(&self, symbols: &[Symbol]) -> HashMap<String, Vec<String>> {
        let mut local_map = HashMap::new();
        for sym in symbols {
            local_map
                .entry(sym.name.clone())
                .or_insert_with(Vec::new)
                .push(sym.hash());
        }
        local_map
    }

    fn insert_new_symbols(
        &mut self,
        new_symbols: &[Symbol],
        global_name_map: &mut HashMap<String, Vec<String>>,
    ) {
        for sym in new_symbols {
            let sym_hash = sym.hash();

            // (Use the "safe" add_symbol method to store the symbol
            self.symbol_store.add_symbol(sym.clone());

            // Also update the global name_map for cross-file lookups
            global_name_map
                .entry(sym.name.clone())
                .or_default()
                .push(sym_hash);
        }
    }

    fn resolve_one_dependency(
        &self,
        raw_name: &str,
        local_name_map: &HashMap<String, Vec<String>>,
        global_name_map: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        if let Some(local_candidates) = local_name_map.get(raw_name) {
            local_candidates.clone()
        } else if let Some(global_candidates) = global_name_map.get(raw_name) {
            global_candidates.clone()
        } else {
            Vec::new()
        }
    }

    fn resolve_new_symbols_dependencies(
        &mut self,
        new_symbols: &[Symbol],
        local_name_map: &HashMap<String, Vec<String>>,
        global_name_map: &HashMap<String, Vec<String>>,
        file_path: &str,
    ) {
        for sym in new_symbols {
            let this_hash = sym.hash();

            // Remove the symbol from the index so we have full ownership,
            if let Some(mut updated_sym) = self.symbol_store.remove_symbol(&this_hash) {
                // Take the old raw dependency names from `updated_sym`
                let old_deps = take(&mut updated_sym.dependencies);

                // We'll store the newly hashed references here
                let mut new_dep_hashes = HashSet::new();

                // For each raw dependency name, see if it’s local or global
                for raw_name in old_deps {
                    let candidates =
                        self.resolve_one_dependency(&raw_name, local_name_map, global_name_map);
                    if candidates.is_empty() {
                        warn!(
                            "Dependency '{}' not found for symbol '{}'. (File: {})",
                            raw_name, updated_sym.name, file_path
                        );
                        // Store it in unresolved, in case we parse it later
                        // self.add_unresolved_dep(this_hash.clone(), raw_name);
                        self.dependency_resolver.add(this_hash.clone(), raw_name);
                    } else {
                        for dep_hash in candidates {
                            if dep_hash != this_hash {
                                new_dep_hashes.insert(dep_hash.clone());
                                self.symbol_store.add_used_by(&dep_hash, &this_hash);
                            }
                        }
                    }
                }

                // Store the hashed dependencies in `updated_sym`
                updated_sym.dependencies = new_dep_hashes.into_iter().collect();

                // Reinsert the symbol into the index
                self.symbol_store.add_symbol(updated_sym);
            }
        }
    }

    // ----------------- Unresolved Deps -----------------

    /// Attempt to recheck the unresolved references. Any that can now be found
    /// (because we've parsed more files) will be resolved; leftover remain unresolved.
    pub fn recheck_unresolved(&mut self) {
        // We'll move the entire map out of self so we don't hold a mutable borrow of unresolved_deps
        let drained: Vec<(String, Vec<String>)> = self.dependency_resolver.collect_drained();
        let mut still_unresolved = HashMap::new();

        // Build a name map once here. If you expect the name map to change as we fix references,
        // you can rebuild it inside the loop or after each fix. But typically once at the start is enough.
        let global_map = self.symbol_store.build_name_map();

        // For each (caller_symbol_hash, list_of_missing_names)
        for (caller_hash, missing_names) in drained {
            // We remove the caller symbol so we can safely mutate it offline
            if let Some(mut caller_sym) = self.symbol_store.remove_symbol(&caller_hash) {
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
                self.symbol_store.add_symbol(caller_sym);

                // Now that caller_sym is no longer borrowed, we can safely call `add_used_by`.
                for (dep_h, caller_h) in used_by_links {
                    self.symbol_store.add_used_by(&dep_h, &caller_h);
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
        self.dependency_resolver
            .replace_unresolved(still_unresolved);
    }
}
