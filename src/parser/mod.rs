pub mod language_config;

use language_config::LanguageConfig;
use tree_sitter::{Node, Parser};

use crate::indexer::symbol::Symbol;

pub struct CodeParser {
    parser: Parser,
    config: LanguageConfig,
}

impl CodeParser {
    pub fn new_rust() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .expect("Error loading Rust grammar.");

        CodeParser {
            parser,
            config: LanguageConfig::rust_config(),
        }
    }

    pub fn parse_file(&mut self, file_path: &str) -> Vec<Symbol> {
        let code = match std::fs::read(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to read file {}: {}", file_path, e);
                return Vec::new();
            }
        };

        let tree = match self.parser.parse(&code, None) {
            Some(t) => t,
            None => {
                eprintln!("Failed to parse file {}", file_path);
                return Vec::new();
            }
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();
        extract_symbols(root_node, &code, file_path, &mut symbols, &self.config);
        symbols
    }
}

pub fn extract_symbols(
    node: Node,
    code: &[u8],
    file_path: &str,
    symbols: &mut Vec<Symbol>,
    config: &LanguageConfig,
) {
    if let Some(symbol_type) = config.kind_to_symbol.get(node.kind()) {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(code) {
                let start_position = name_node.start_position();
                symbols.push(Symbol {
                    name: name.to_string(),
                    symbol_type: symbol_type.clone(),
                    file_path: file_path.to_string(),
                    line_number: start_position.row + 1,
                    start_byte: name_node.start_byte(),
                    end_byte: name_node.end_byte(),
                    dependencies: vec![],
                });
                println!("Extracted symbol: {}", name);
            } else {
                eprintln!("Failed to extract text for name node in {}", file_path);
            }
        }
    }

    // Recurse into child nodes
    for child in node.children(&mut node.walk()) {
        extract_symbols(child, code, file_path, symbols, config);
    }
}