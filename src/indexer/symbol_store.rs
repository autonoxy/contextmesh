use crate::symbol::Symbol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SymbolStore {
    /// Maps unique symbol hashes -> their Symbol structure
    symbols: HashMap<String, Symbol>,
}

impl SymbolStore {
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn add_symbol(&mut self, symbol: Symbol) -> Option<Symbol> {
        let hash = symbol.hash();
        self.symbols.insert(hash, symbol)
    }

    pub fn remove_symbol(&mut self, sym_hash: &str) -> Option<Symbol> {
        let removed_sym = self.symbols.remove(sym_hash);

        if removed_sym.is_some() {
            for s in self.symbols.values_mut() {
                s.used_by.remove(sym_hash);
            }
        }

        removed_sym
    }

    pub fn get_symbols(&self) -> &HashMap<String, Symbol> {
        &self.symbols
    }

    pub fn add_used_by(&mut self, callee_hash: &str, caller_hash: &str) -> bool {
        if let Some(sym) = self.symbols.get_mut(callee_hash) {
            sym.used_by.insert(caller_hash.to_string());
            true
        } else {
            false
        }
    }

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
