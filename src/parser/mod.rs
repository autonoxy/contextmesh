mod language_kinds;

use crate::indexer::symbol::Symbol;
use crate::parser::language_kinds::language_kinds_map;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

pub struct CodeParser {
    parser: Parser,
}

impl CodeParser {
    pub fn new_rust() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .expect("Error loading Rust grammar.");

        CodeParser { parser }
    }

    pub fn parse_file(
        &mut self,
        file_path: &str,
        language: &str,
    ) -> (Vec<Symbol>, HashMap<String, String>) {
        let code = match std::fs::read(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to read file {}: {}", file_path, e);
                return (Vec::new(), HashMap::new());
            }
        };

        let tree = match self.parser.parse(&code, None) {
            Some(t) => t,
            None => {
                eprintln!("Failed to parse file {}", file_path);
                return (Vec::new(), HashMap::new());
            }
        };

        let root_node = tree.root_node();

        let mut symbols = Vec::new();
        let mut imports = HashMap::new();

        let map = language_kinds_map();
        let allowed_kinds = match map.get(language) {
            Some(kinds) => kinds,
            None => {
                eprintln!("No known node kinds for language: {}", language);
                return (Vec::new(), HashMap::new());
            }
        };

        collect_definitions_and_imports(
            root_node,
            &code,
            file_path,
            &mut symbols,
            &mut imports,
            allowed_kinds,
        );

        let mut symbol_stack = Vec::new();
        gather_references(
            root_node,
            &code,
            file_path,
            &mut symbols,
            &imports,
            &mut symbol_stack,
        );

        (symbols, imports)
    }
}

/// Walk the AST once to collect symbol definitions (any node with a `name` field)
/// plus 'use_declaration' nodes, storing them in `imports`.
fn collect_definitions_and_imports(
    node: Node,
    code: &[u8],
    file_path: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut HashMap<String, String>,
    allowed_kinds: &std::collections::HashSet<&str>,
) {
    let node_kind = node.kind();

    // 1) If node has a `name` field, treat it as a symbol definition
    if let Some(name_node) = node.child_by_field_name("name") {
        if allowed_kinds.contains(node_kind) {
            if let Ok(name) = name_node.utf8_text(code) {
                let start_pos = name_node.start_position();
                symbols.push(Symbol {
                    name: name.to_string(),
                    node_kind: node_kind.to_string(),
                    file_path: file_path.to_string(),
                    line_number: start_pos.row + 1,
                    start_byte: name_node.start_byte(),
                    end_byte: name_node.end_byte(),
                    dependencies: vec![],
                    used_by: vec![],
                });
            }
        }
    }

    // 2) If node is a 'use_declaration', parse out the path/alias
    //    Tree-sitter Rust: (use_declaration ... (use_list)?) or a single path
    if node_kind == "use_declaration" {
        // We'll do a naive approach:
        // look for child 'name' (alias) or parse the path text directly
        // e.g. `use crate::foo::Bar as MyBar;`
        // We'll store something like `imports.insert("MyBar", "crate::foo::Bar");`
        // This entire text might be "use crate::foo::Bar as MyBar;"
        // We can do a simpler parse approach, or a more structured approach
        // Let's do a structured approach with child_by_field_name("alias"), "path", etc.

        if let Some(path_node) = node.child_by_field_name("path") {
            if let Ok(path_text) = path_node.utf8_text(code) {
                let mut alias_name = path_text.to_string();
                if let Some(alias_node) = node.child_by_field_name("alias") {
                    if let Ok(a_text) = alias_node.utf8_text(code) {
                        alias_name = a_text.to_string(); // The alias
                    }
                }
                // Now store in imports map:
                // key = alias_name, value = full path (or path_text)
                // This is naive but works for common cases
                imports.insert(alias_name, path_text.to_string());
            }
        }
    }

    // Recurse
    for child in node.children(&mut node.walk()) {
        collect_definitions_and_imports(child, code, file_path, symbols, imports, allowed_kinds);
    }
}

/// Walk the AST a second time to gather references:
/// - call_expression
/// - method_call_expression
///     Possibly naive path references
///     We'll keep a stack of "current symbol" so we know who is referencing what.
fn gather_references(
    node: Node,
    code: &[u8],
    file_path: &str,
    symbols: &mut [Symbol],
    imports: &HashMap<String, String>,
    symbol_stack: &mut Vec<usize>,
) {
    let node_kind = node.kind();

    // If there's a 'name' field, this is a new symbol scope
    if let Some(name_node) = node.child_by_field_name("name") {
        if name_node.utf8_text(code).is_ok() {
            // Find the index of the symbol we created in the first pass
            // We'll do a quick linear search. We rely on (node_kind + line_number + file_path) matching
            // to find the correct symbol index. You might store line_number in the first pass.
            let start_pos = name_node.start_position();
            if let Some((idx, _found_sym)) = symbols.iter().enumerate().find(|(_, s)| {
                s.file_path == file_path
                    && s.line_number == start_pos.row + 1
                    && s.node_kind == node_kind
            }) {
                symbol_stack.push(idx);

                // Recurse
                for child in node.children(&mut node.walk()) {
                    gather_references(child, code, file_path, symbols, imports, symbol_stack);
                }

                symbol_stack.pop();
                return;
            }
        }
    }

    // Check for call expressions
    if node_kind == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            // Could be an identifier, field_expression, scoped_identifier, etc.
            let func_str = extract_callable_name(func_node, code, imports);
            if let Some(&parent_idx) = symbol_stack.last() {
                symbols[parent_idx].dependencies.push(func_str);
            }
        }
    }
    // Also handle method_call_expression (like foo.bar(...)):
    else if node_kind == "method_call_expression" {
        // Tree-sitter Rust: method_call_expression has child_by_field_name("method") for the method name
        if let Some(method_node) = node.child_by_field_name("method") {
            let method_str = method_node.utf8_text(code).unwrap_or_default().to_string();
            // If there's an import alias for method_str, you'd do extra logic here,
            // but typically method calls won't be aliased.
            if let Some(&parent_idx) = symbol_stack.last() {
                symbols[parent_idx].dependencies.push(method_str);
            }
        }
    }

    // You might also handle 'identifier' nodes referencing a variable or function,
    // or 'scoped_identifier' for `foo::bar`
    // We'll skip for brevity.

    // Recurse
    for child in node.children(&mut node.walk()) {
        gather_references(child, code, file_path, symbols, imports, symbol_stack);
    }
}

/// Utility to parse out the function name from a node that might be an identifier
/// or a path that references an alias. We'll do a naive approach:
/// 1. If it's an 'identifier', get its text.
/// 2. If it matches an import alias, replace with the import's fully qualified path.
fn extract_callable_name(node: Node, code: &[u8], imports: &HashMap<String, String>) -> String {
    let node_kind = node.kind();
    if node_kind == "identifier" {
        // e.g. "foo"
        let text = node.utf8_text(code).unwrap_or("unknown");
        // If there's an import alias for "foo", substitute
        if let Some(full_path) = imports.get(text) {
            full_path.clone()
        } else {
            text.to_string()
        }
    } else if node_kind == "scoped_identifier" {
        // e.g. "crate::foo::bar"
        node.utf8_text(code).unwrap_or("unknown_scoped").to_string()
    } else if node_kind == "field_expression" {
        // e.g. "my_struct.foo"
        // you'd parse deeper. We'll just read the raw text
        node.utf8_text(code).unwrap_or("field_expr").to_string()
    } else {
        // fallback
        node.utf8_text(code)
            .unwrap_or("unknown_callable")
            .to_string()
    }
}
