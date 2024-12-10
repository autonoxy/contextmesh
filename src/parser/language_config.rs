use std::collections::HashMap;

use crate::indexer::symbol::SymbolType;

pub struct LanguageConfig {
    pub kind_to_symbol: HashMap<&'static str, SymbolType>,
}

impl LanguageConfig {
    pub fn rust_config() -> Self {
        let mut kind_to_symbol = HashMap::new();
        kind_to_symbol.insert("function_item", SymbolType::Function);
        kind_to_symbol.insert("struct_item", SymbolType::Struct);
        kind_to_symbol.insert("enum_item", SymbolType::Enum);
        kind_to_symbol.insert("use_declaration", SymbolType::Import);
        kind_to_symbol.insert("let_declaration", SymbolType::Variable);
        kind_to_symbol.insert("field_declaration", SymbolType::Field);
        kind_to_symbol.insert("enum_variant", SymbolType::Field);

        LanguageConfig { kind_to_symbol }
    }
}