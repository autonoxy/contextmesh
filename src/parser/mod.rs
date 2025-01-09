pub mod language; // The trait
pub mod rust_indexer; // The Rust plugin

use crate::{errors::ContextMeshError, indexer::symbol::Symbol};
use language::LanguageIndexer;
use rust_indexer::RustIndexer;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

pub struct CodeParser {
    parser: Parser,
    plugin: Box<dyn LanguageIndexer>, // Language-specific implementation
}

impl CodeParser {
    pub fn new_rust() -> Result<Self, ContextMeshError> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .map_err(|_| {
                ContextMeshError::TreeSitterError("Failed to set Rust language.".to_string())
            })?;

        Ok(CodeParser {
            parser,
            plugin: Box::new(RustIndexer),
        })
    }

    /// Parse a single file, returning (symbols, imports).
    pub fn parse_file(
        &mut self,
        file_path: &str,
    ) -> Result<(Vec<Symbol>, HashMap<String, String>), ContextMeshError> {
        println!(
            "Parsing file '{}' using {} indexer...",
            file_path,
            self.plugin.language_name()
        );

        let code = std::fs::read(file_path).map_err(|e| {
            eprintln!("Failed to read file {}: {}", file_path, e);
            ContextMeshError::TreeSitterError("Parsing returned no tree.".to_string())
        })?;

        let tree = self.parser.parse(&code, None).ok_or_else(|| {
            eprintln!("Failed to parse file {}.", file_path);
            ContextMeshError::TreeSitterError("Parsing returned no tree.".to_string())
        })?;

        let root = tree.root_node();

        let mut symbols = Vec::new();
        let mut imports = HashMap::new();

        // Initialize module stack
        let mut current_module = Vec::new();

        // 1) Collect definitions + imports in one pass
        let _ = collect_definitions_and_imports(
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
        let _ = gather_references(
            &*self.plugin,
            root,
            &code,
            file_path,
            &mut symbols,
            &imports,
            &mut symbol_stack,
        );

        Ok((symbols, imports))
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
) -> Result<(), ContextMeshError> {
    // Enter module if applicable
    lang.enter_module(node, code, current_module)?;

    let node_kind = node.kind();

    // If it's an import node, pass it to the language plugin:
    lang.process_import_declaration(node, code, imports)?;

    // If node kind is in `allowed_definition_kinds`, build a Symbol:
    if lang.allowed_definition_kinds().contains(&node_kind) {
        let full_name = lang.build_qualified_name(node, code)?;
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
        )?;
    }

    // Exit module if applicable
    lang.exit_module(current_module)?;

    Ok(())
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
) -> Result<(), ContextMeshError> {
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
                gather_references(lang, child, code, file_path, symbols, imports, symbol_stack)?;
            }

            symbol_stack.pop();
            return Ok(());
        }
    }

    // Check for call expressions
    if node_kind == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            match lang.extract_callable_name(func_node, code, imports) {
                Ok(call_name) => {
                    if let Some(&parent_idx) = symbol_stack.last() {
                        symbols[parent_idx].dependencies.push(call_name);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to extract callable name in file '{}': {}",
                        file_path, e
                    );
                    // Depending on requirements, you might choose to continue or return
                }
            }
        }
    }
    // Also handle method_call_expression (like foo.bar(...)):
    else if node_kind == "method_call_expression" {
        // Tree-sitter Rust: method_call_expression has child_by_field_name("method") for the method name
        if let Some(method_node) = node.child_by_field_name("method") {
            match method_node.utf8_text(code) {
                Ok(method_str) => {
                    if let Some(&parent_idx) = symbol_stack.last() {
                        symbols[parent_idx]
                            .dependencies
                            .push(method_str.to_string());
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to extract method name in file '{}': {}",
                        file_path, e
                    );
                }
            }
        }
    }

    // Recurse children
    for child in node.children(&mut node.walk()) {
        gather_references(lang, child, code, file_path, symbols, imports, symbol_stack)?;
    }

    Ok(())
}
