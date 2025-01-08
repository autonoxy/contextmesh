pub mod language; // The trait
pub mod rust_indexer; // The Rust plugin

use crate::indexer::symbol::Symbol;
use language::LanguageIndexer;
use rust_indexer::RustIndexer;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

pub struct CodeParser {
    parser: Parser,
    plugin: Box<dyn LanguageIndexer>, // Language-specific implementation
}

impl CodeParser {
    pub fn new_rust() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .expect("Error loading Rust grammar.");

        CodeParser {
            parser,
            plugin: Box::new(RustIndexer),
        }
    }

    /// Parse a single file, returning (symbols, imports).
    pub fn parse_file(&mut self, file_path: &str) -> (Vec<Symbol>, HashMap<String, String>) {
        println!(
            "Parsing file '{}' using {} indexer...",
            file_path,
            self.plugin.language_name()
        );

        let code = match std::fs::read(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read file {}: {}", file_path, e);
                return (vec![], HashMap::new());
            }
        };

        let tree = match self.parser.parse(&code, None) {
            Some(t) => t,
            None => {
                eprintln!("Failed to parse file {}", file_path);
                return (vec![], HashMap::new());
            }
        };

        let root = tree.root_node();

        let mut symbols = Vec::new();
        let mut imports = HashMap::new();

        // Initialize module stack
        let mut current_module = Vec::new();

        // 1) Collect definitions + imports in one pass
        collect_definitions_and_imports(
            &*self.plugin,
            root,
            &code,
            file_path,
            &mut symbols,
            &mut imports,
            &mut current_module,
        );

        // 2) Gather references
        let mut symbol_stack = Vec::new();
        gather_references(
            &*self.plugin,
            root,
            &code,
            file_path,
            &mut symbols,
            &imports,
            &mut symbol_stack,
        );

        (symbols, imports)
    }
}

/// Traverse the AST to collect definitions and imports.
fn collect_definitions_and_imports(
    lang: &dyn LanguageIndexer,
    node: Node,
    code: &[u8],
    file_path: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut HashMap<String, String>,
    current_module: &mut Vec<String>,
) {
    // Enter module if applicable
    lang.enter_module(node, code, current_module);

    let node_kind = node.kind();

    // If it's an import node, pass it to the language plugin:
    lang.process_import_declaration(node, code, imports);

    // If node kind is in `allowed_definition_kinds`, build a Symbol:
    if lang.allowed_definition_kinds().contains(&node_kind) {
        if let Some(full_name) = lang.build_qualified_name(node, code) {
            let start = node.start_position();
            symbols.push(Symbol {
                name: full_name,
                node_kind: node_kind.to_string(),
                file_path: file_path.to_string(),
                line_number: start.row + 1,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                dependencies: vec![],
                used_by: vec![],
            });
        }
    }

    // Recurse children
    for child in node.children(&mut node.walk()) {
        collect_definitions_and_imports(
            lang,
            child,
            code,
            file_path,
            symbols,
            imports,
            current_module,
        );
    }

    // Exit module if applicable
    lang.exit_module(current_module);
}

/// Traverse the AST to gather references.
fn gather_references(
    lang: &dyn LanguageIndexer,
    node: Node,
    code: &[u8],
    file_path: &str,
    symbols: &mut Vec<Symbol>,
    imports: &HashMap<String, String>,
    symbol_stack: &mut Vec<usize>,
) {
    let node_kind = node.kind();

    // If there's a 'name' field, this might be a new symbol scope
    if let Some(name_node) = node.child_by_field_name("name") {
        let start = name_node.start_position();
        if let Some((idx, _sym)) = symbols.iter().enumerate().find(|(_, s)| {
            s.file_path == file_path && s.line_number == start.row + 1 && s.node_kind == node_kind
        }) {
            symbol_stack.push(idx);

            // Recurse children
            for child in node.children(&mut node.walk()) {
                gather_references(lang, child, code, file_path, symbols, imports, symbol_stack);
            }

            symbol_stack.pop();
            return;
        }
    }

    // Check for call expressions
    if node_kind == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            if let Some(call_name) = lang.extract_callable_name(func_node, code, imports) {
                if let Some(&parent_idx) = symbol_stack.last() {
                    symbols[parent_idx].dependencies.push(call_name);
                }
            }
        }
    }
    // Also handle method_call_expression (like foo.bar(...)):
    else if node_kind == "method_call_expression" {
        // Tree-sitter Rust: method_call_expression has child_by_field_name("method") for the method name
        if let Some(method_node) = node.child_by_field_name("method") {
            let method_str = method_node.utf8_text(code).unwrap_or_default().to_string();
            if let Some(&parent_idx) = symbol_stack.last() {
                symbols[parent_idx].dependencies.push(method_str);
            }
        }
    }

    // Recurse children
    for child in node.children(&mut node.walk()) {
        gather_references(lang, child, code, file_path, symbols, imports, symbol_stack);
    }
}
