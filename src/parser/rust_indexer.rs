use crate::errors::ContextMeshError;

use super::language::LanguageIndexer;
use std::collections::HashMap;
use tree_sitter::Node;

/// Rust-specific implementation of the `LanguageIndexer` trait.
///
/// The `RustIndexer` struct provides methods to parse Rust code, extract symbols,
/// handle imports, and manage module scopes. It leverages the Tree-sitter parser
/// to navigate the Abstract Syntax Tree (AST) of Rust source files.
pub struct RustIndexer;

impl LanguageIndexer for RustIndexer {
    /// Returns the name of the language that this indexer handles.
    fn language_name(&self) -> &'static str {
        "rust"
    }

    /// Provides a list of node kinds that represent top-level definitions in Rust.
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

    /// Constructs the fully qualified name of a Rust symbol given its AST node.
    fn build_qualified_name(&self, node: Node, code: &[u8]) -> Result<String, ContextMeshError> {
        // Extract the symbol's short name
        if let Some(name_node) = node.child_by_field_name("name") {
            let short_name = name_node.utf8_text(code).map_err(|_| {
                ContextMeshError::DeserializationError("Failed to extract name text.".to_string())
            })?;

            Ok(short_name.to_string())
        } else {
            Err(ContextMeshError::DeserializationError(
                "Skipping empty-named item.".to_string(),
            ))
        }
    }

    /// Parses Rust import declarations (`use` statements) to populate the `imports` map.
    fn process_import_declaration(
        &self,
        node: Node,
        code: &[u8],
        imports: &mut HashMap<String, String>,
    ) -> Result<(), ContextMeshError> {
        if node.kind() != "use_declaration" {
            return Ok(());
        }

        // Handle 'use' declarations with potential aliases
        // e.g., use crate::foo::Bar as Baz;
        // or use crate::foo::Bar;

        // Extract the path
        if let Some(path_node) = node.child_by_field_name("path") {
            let path_text = path_node
                .utf8_text(code)
                .map_err(|_| {
                    ContextMeshError::DeserializationError(
                        "Failed to extract path text.".to_string(),
                    )
                })?
                .to_string();

            // Check for an alias
            if let Some(alias_node) = node.child_by_field_name("alias") {
                let alias_text = alias_node
                    .utf8_text(code)
                    .map_err(|_| {
                        ContextMeshError::DeserializationError(
                            "Failed to extract alias text.".to_string(),
                        )
                    })?
                    .to_string();
                imports.insert(alias_text.to_string(), path_text);
            } else {
                // No alias; insert the last segment as the identifier
                if let Some(last_segment) = path_text.split("::").last() {
                    imports.insert(last_segment.to_string(), path_text);
                }
            }
        }

        Ok(())
    }

    /// Extracts the name of a callable entity (e.g., function, method) from a reference node.
    fn extract_callable_name(
        &self,
        node: Node,
        code: &[u8],
        imports: &HashMap<String, String>,
    ) -> Result<String, ContextMeshError> {
        let node_kind = node.kind();
        match node_kind {
            "identifier" => {
                let text = node.utf8_text(code).map_err(|_| {
                    ContextMeshError::DeserializationError(
                        "Failed to extract identifier text.".to_string(),
                    )
                })?;
                // Replace with full path if alias exists
                if let Some(full_path) = imports.get(text) {
                    Ok(full_path.clone())
                } else {
                    Ok(text.to_string())
                }
            }
            "scoped_identifier" => {
                // e.g., "commands::run_command"
                let raw = node.utf8_text(code).map_err(|_| {
                    ContextMeshError::DeserializationError(
                        "Failed to extract scoped identifier text.".to_string(),
                    )
                })?;

                Ok(raw
                    .split("::")
                    .last()
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        ContextMeshError::DeserializationError(
                            "Failed to extract last segment of scoped identifier.".to_string(),
                        )
                    })?)
            }
            _ => Ok(String::new()),
        }
    }

    /// Handles entering a new module or namespace scope during parsing.
    fn enter_module(
        &self,
        node: Node,
        code: &[u8],
        current_module: &mut Vec<String>,
    ) -> Result<(), ContextMeshError> {
        if node.kind() == "mod_item" {
            // Extract module name
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node
                    .utf8_text(code)
                    .map_err(|_| {
                        ContextMeshError::DeserializationError(
                            "Failed to extract module name.".to_string(),
                        )
                    })?
                    .to_string();
                current_module.push(name);
            }
        }
        Ok(())
    }

    /// Handles exiting a module or namespace scope during parsing.
    fn exit_module(&self, current_module: &mut Vec<String>) -> Result<(), ContextMeshError> {
        if !current_module.is_empty() {
            current_module.pop();
        }
        Ok(())
    }
}
