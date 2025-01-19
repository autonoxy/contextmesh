use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::mem::take;

use crate::errors::ContextMeshError;
use crate::indexer::{calculate_file_hash, symbol::Symbol, Indexer};
use crate::parser::CodeParser;
use crate::utils::collect_files;

/// The public entry point for indexing (truly incremental).
pub fn handle_index(dir_or_file: &str, language: &str) -> Result<(), ContextMeshError> {
    // 1) Ensure .contextmesh directory
    ensure_index_directory_exists(".contextmesh")?;
    let mut indexer = load_or_create_index()?;

    // 2) Prepare parser
    let (extensions, mut code_parser) = prepare_parser(language)?;

    // 3) Gather all candidate files (based on extension)
    let files = collect_files(dir_or_file, extensions);

    // 4) Build a global name->list_of_hashes map from existing index
    let mut global_name_map = indexer.build_name_map();

    // 5) Process each changed file individually
    for file_path in files {
        let new_hash = match calculate_file_hash(&file_path) {
            Some(h) => h,
            None => {
                warn!("Could not read/hash file '{}'. Skipping.", file_path);
                continue;
            }
        };

        // Only parse if changed
        if indexer.has_changed(&file_path, &new_hash) {
            info!("File '{}' changed. Parsing now...", file_path);

            // Remove old/renamed/deleted symbols from this file
            remove_deleted_symbols_in_file(&mut indexer, &file_path, &mut global_name_map);

            parse_and_index_file(
                &file_path,
                &new_hash,
                &mut indexer,
                &mut code_parser,
                &mut global_name_map,
            )?;
        } else {
            debug!("File '{}' is up-to-date. Skipping parse.", file_path);
        }
    }

    // 6) Re-check any unresolved references (forward references, etc.)
    indexer.recheck_unresolved();

    // 7) Save the updated index
    indexer.save_index()?;
    info!("Incremental index updated successfully.");
    println!("Index updated successfully.");

    Ok(())
}

// ----------------------------------------------------------------------
// Step 1: Ensure .contextmesh and Load or Create Index
// ----------------------------------------------------------------------

fn ensure_index_directory_exists(path: &str) -> Result<(), ContextMeshError> {
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir_all(path)?;
        info!("Created directory: {}", path);
    }
    Ok(())
}

fn load_or_create_index() -> Result<Indexer, ContextMeshError> {
    println!("Loading existing index...");
    match Indexer::load_index() {
        Ok(existing) => Ok(existing),
        Err(e) => {
            warn!("No existing index found (or failed to load): {e}. Creating a new one.");
            Ok(Indexer::new())
        }
    }
}

// ----------------------------------------------------------------------
// Step 2: Prepare Parser
// ----------------------------------------------------------------------

fn prepare_parser(
    language: &str,
) -> Result<(&'static [&'static str], CodeParser), ContextMeshError> {
    // 1) Initialize code parser
    let code_parser = match language.to_lowercase().as_str() {
        "rust" => CodeParser::new_rust().map_err(|e| {
            eprintln!(
                "Failed to initialize CodeParser for language '{}': {}",
                language, e
            );
            e
        })?,
        _ => {
            eprintln!("Unsupported language: {}", language);
            return Err(ContextMeshError::UnsupportedLanguage(language.to_string()));
        }
    };

    // 2) Determine extensions
    let extensions = match language.to_lowercase().as_str() {
        "rust" => &["rs"],
        _ => return Err(ContextMeshError::UnsupportedLanguage(language.to_string())),
    };

    Ok((extensions, code_parser))
}

// ----------------------------------------------------------------------
// Step 5: Parse + Cleanup + Index a Single Changed File
// ----------------------------------------------------------------------

/// Remove old symbols from this file if they no longer exist or have been renamed.
/// Also remove them from the `global_name_map`.
fn remove_deleted_symbols_in_file(
    indexer: &mut Indexer,
    file_path: &str,
    global_name_map: &mut HashMap<String, Vec<String>>,
) {
    // Gather all existing symbols from this file
    let old_symbols: Vec<(String, String)> = indexer
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
        indexer.remove_symbol(&old_hash);

        if let Some(vec_of_hashes) = global_name_map.get_mut(&old_name) {
            vec_of_hashes.retain(|h| h != &old_hash);
            if vec_of_hashes.is_empty() {
                global_name_map.remove(&old_name);
            }
        }
    }
}

fn parse_and_index_file(
    file_path: &str,
    new_hash: &str,
    indexer: &mut Indexer,
    code_parser: &mut CodeParser,
    global_name_map: &mut HashMap<String, Vec<String>>,
) -> Result<(), ContextMeshError> {
    // 1) Parse the file => new symbols
    let new_symbols = parse_file_symbols(file_path, code_parser)?;

    // 2) Build local name->hash for references *within* this file
    let local_name_map = build_local_name_map(&new_symbols);

    // 3) Insert new symbols into the index & update global name map
    insert_new_symbols(&new_symbols, indexer, global_name_map);

    // 4) Resolve dependencies right away, linking to local or global symbols
    resolve_new_symbols_dependencies(
        &new_symbols,
        indexer,
        &local_name_map,
        global_name_map,
        file_path,
    );

    // 5) Mark the file hash so we know it's up to date next run
    indexer.store_file_hash(file_path, new_hash);

    debug!("Finished incremental update for '{}'.", file_path);
    Ok(())
}

// ---- Step 5.1: Parse the file to get new symbols
fn parse_file_symbols(
    file_path: &str,
    code_parser: &mut CodeParser,
) -> Result<Vec<Symbol>, ContextMeshError> {
    let (parsed_syms, _imports) = code_parser.parse_file(file_path)?;
    debug!("Parsed {} symbols from '{}'.", parsed_syms.len(), file_path);
    Ok(parsed_syms)
}

// ---- Step 5.2: Build local name->Vec<hash>
fn build_local_name_map(symbols: &[Symbol]) -> HashMap<String, Vec<String>> {
    let mut local_map = HashMap::new();
    for sym in symbols {
        local_map
            .entry(sym.name.clone())
            .or_insert_with(Vec::new)
            .push(sym.hash());
    }
    local_map
}

// ---- Step 5.3: Insert newly parsed symbols into the index, updating global name_map
fn insert_new_symbols(
    new_symbols: &[Symbol],
    indexer: &mut Indexer,
    global_name_map: &mut HashMap<String, Vec<String>>,
) {
    for sym in new_symbols {
        let sym_hash = sym.hash();

        // (A) Use the "safe" add_symbol method to store the symbol
        indexer.add_symbol(sym.clone());

        // (B) Also update the global name_map for cross-file lookups
        global_name_map
            .entry(sym.name.clone())
            .or_default()
            .push(sym_hash);
    }
}

// ---- Step 5.4: Resolve dependencies for each new symbol
fn resolve_new_symbols_dependencies(
    new_symbols: &[Symbol],
    indexer: &mut Indexer,
    local_name_map: &HashMap<String, Vec<String>>,
    global_name_map: &HashMap<String, Vec<String>>,
    file_path: &str,
) {
    for sym in new_symbols {
        let this_hash = sym.hash();

        // 1) Remove the symbol from the index so we have full ownership,
        //    avoiding double mutable borrows.
        if let Some(mut updated_sym) = indexer.remove_symbol(&this_hash) {
            // 2) Take the old raw dependency names from `updated_sym`
            let old_deps = take(&mut updated_sym.dependencies);

            // 3) We'll store the newly hashed references here
            let mut new_dep_hashes = HashSet::new();

            // 4) For each raw dependency name, see if it’s local or global
            for raw_name in old_deps {
                let candidates = resolve_one_dependency(&raw_name, local_name_map, global_name_map);
                if candidates.is_empty() {
                    warn!(
                        "Dependency '{}' not found for symbol '{}'. (File: {})",
                        raw_name, updated_sym.name, file_path
                    );
                    // Store it in unresolved, in case we parse it later
                    indexer.add_unresolved_dep(this_hash.clone(), raw_name);
                } else {
                    for dep_hash in candidates {
                        if dep_hash != this_hash {
                            new_dep_hashes.insert(dep_hash.clone());
                            indexer.add_used_by(&dep_hash, &this_hash);
                        }
                    }
                }
            }

            // 5) Store the hashed dependencies in `updated_sym`
            updated_sym.dependencies = new_dep_hashes.into_iter().collect();

            // 6) Reinsert the symbol into the index
            indexer.add_symbol(updated_sym);
        }
    }
}

// ----------------------------------------------------------------------
// A small utility: check local first, then global. Return all candidate hashes.
// ----------------------------------------------------------------------
fn resolve_one_dependency(
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
