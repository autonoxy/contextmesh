use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DependencyResolver {
    /// Records references that can't be resolved yet (e.g., forward references).
    /// Key = caller hash symbol, Value = list of raw names that don't exist yet.
    unresolved_deps: HashMap<String, Vec<String>>,
}

impl DependencyResolver {
    pub fn len(&self) -> usize {
        self.unresolved_deps.len()
    }

    pub fn add(&mut self, caller_hash: String, missing_name: String) {
        self.unresolved_deps
            .entry(caller_hash)
            .or_default()
            .push(missing_name);
    }

    pub fn collect_drained(&mut self) -> Vec<(String, Vec<String>)> {
        self.unresolved_deps.drain().collect()
    }

    pub fn replace_unresolved(&mut self, deps: HashMap<String, Vec<String>>) {
        self.unresolved_deps = deps
    }
}
