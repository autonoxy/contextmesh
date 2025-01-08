use super::language::LanguageIndexer;
use std::collections::HashMap;
use tree_sitter::Node;

/// Rust-specific implementation of the LanguageIndexer trait.
pub struct RustIndexer;

impl LanguageIndexer for RustIndexer {
    fn language_name(&self) -> &'static str {
        "rust"
    }

    fn allowed_definition_kinds(&self) -> &'static [&'static str] {
        &[
            "function_item",
            "method_declaration",
            "trait_item",
            "impl_item",
            "struct_item",
            "enum_item",
            "field_declaration",
            "static_item",
            "const_item",
        ]
    }

    fn build_qualified_name(
        &self,
        current_module: &Vec<String>,
        node: Node,
        code: &[u8],
    ) -> Option<String> {
        // Extract the symbol's short name
        let name_node = node.child_by_field_name("name")?;
        let short_name = name_node.utf8_text(code).ok()?;

        // Combine with current module path
        if current_module.is_empty() {
            Some(short_name.to_string())
        } else {
            Some(format!("{}::{}", current_module.join("::"), short_name))
        }
    }

    fn process_import_declaration(
        &self,
        node: Node,
        code: &[u8],
        imports: &mut HashMap<String, String>,
    ) {
        if node.kind() != "use_declaration" {
            return;
        }

        // Handle 'use' declarations with potential aliases
        // e.g., use crate::foo::Bar as Baz;
        // or use crate::foo::Bar;

        // Extract the path
        if let Some(path_node) = node.child_by_field_name("path") {
            let path_text = match path_node.utf8_text(code) {
                Ok(text) => text.to_string(),
                Err(_) => return,
            };

            // Check for an alias
            if let Some(alias_node) = node.child_by_field_name("alias") {
                if let Ok(alias_text) = alias_node.utf8_text(code) {
                    imports.insert(alias_text.to_string(), path_text);
                }
            } else {
                // No alias; insert the last segment as the identifier
                if let Some(last_segment) = path_text.split("::").last() {
                    imports.insert(last_segment.to_string(), path_text);
                }
            }
        }
    }

    fn extract_callable_name(
        &self,
        node: Node,
        code: &[u8],
        imports: &HashMap<String, String>,
    ) -> Option<String> {
        let node_kind = node.kind();
        match node_kind {
            "identifier" => {
                let text = node.utf8_text(code).ok()?;
                // Replace with full path if alias exists
                if let Some(full_path) = imports.get(text) {
                    Some(full_path.clone())
                } else {
                    Some(text.to_string())
                }
            }
            "scoped_identifier" => {
                // e.g., "commands::run_command"
                let raw = node.utf8_text(code).ok()?;
                Some(raw.to_string())
            }
            "field_expression" => {
                // e.g., "my_struct.foo" - typically method calls aren't re-exported
                // Could implement deeper resolution if needed
                None
            }
            _ => None,
        }
    }

    fn enter_module(&self, node: Node, code: &[u8], current_module: &mut Vec<String>) {
        if node.kind() == "mod_item" {
            // Extract module name
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(code) {
                    current_module.push(name.to_string());
                }
            }
        }
    }

    fn exit_module(&self, current_module: &mut Vec<String>) {
        if !current_module.is_empty() {
            current_module.pop();
        }
    }
}
