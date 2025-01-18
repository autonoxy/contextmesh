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
    ///
    /// # Examples
    ///
    /// ```rust
    /// let indexer = RustIndexer;
    /// assert_eq!(indexer.language_name(), "rust");
    /// ```
    ///
    /// # Returns
    ///
    /// A string slice representing the language name, `"rust"`.
    fn language_name(&self) -> &'static str {
        "rust"
    }

    /// Provides a list of node kinds that represent top-level definitions in Rust.
    ///
    /// These node kinds include constructs such as functions, methods, traits,
    /// implementations, structs, enums, fields, statics, and constants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let indexer = RustIndexer;
    /// let kinds = indexer.allowed_definition_kinds();
    /// assert!(kinds.contains(&"function_item"));
    /// ```
    ///
    /// # Returns
    ///
    /// A slice of string slices, each representing a node kind corresponding to
    /// a top-level definition in Rust.
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
    ///
    /// This method extracts the symbol's short name from the AST node and returns
    /// it as a `String`. In more complex scenarios, this method can be extended to
    /// include module or namespace qualifiers to form a fully qualified name.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the symbol definition.
    /// * `code` - The source code as a byte slice, used to extract textual information from the node.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the name node is not found
    /// or if the text extraction fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Assuming `node` corresponds to `struct MyStruct { ... }`
    /// let indexer = RustIndexer;
    /// let code = b"struct MyStruct { ... }";
    /// let qualified_name = indexer.build_qualified_name(node, code)?;
    /// assert_eq!(qualified_name, "MyStruct");
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` containing the symbol's name as a `String` on success,
    /// or a `ContextMeshError` on failure.
    fn build_qualified_name(&self, node: Node, code: &[u8]) -> Result<String, ContextMeshError> {
        // Extract the symbol's short name
        if let Some(name_node) = node.child_by_field_name("name") {
            let short_name = name_node.utf8_text(code).map_err(|_| {
                ContextMeshError::DeserializationError("Failed to extract name text.".to_string())
            })?;

            Ok(short_name.to_string())
        } else {
            Ok(String::new())
        }
    }

    /// Parses Rust import declarations (`use` statements) to populate the `imports` map.
    ///
    /// This method handles `use` declarations with potential aliases, such as:
    ///
    /// - `use crate::foo::Bar as Baz;`
    /// - `use crate::foo::Bar;`
    ///
    /// It maps the alias (if present) or the last segment of the path to the full path.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the import declaration.
    /// * `code` - The source code as a byte slice, used to extract textual information from the node.
    /// * `imports` - A mutable reference to a `HashMap` where aliases or identifiers are mapped to their full paths.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the path or alias text extraction fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Given the import: use crate::foo::Bar as Baz;
    /// let indexer = RustIndexer;
    /// let imports = &mut HashMap::new();
    /// indexer.process_import_declaration(node, code, imports)?;
    /// assert_eq!(imports.get("Baz"), Some(&"crate::foo::Bar".to_string()));
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the import was successfully processed,
    /// or a `ContextMeshError` if an error occurred.
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
    ///
    /// This method handles different node kinds that represent callable references, such as
    /// identifiers and scoped identifiers, resolving them to their fully qualified names using
    /// the provided `imports` map.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the callable reference.
    /// * `code` - The source code as a byte slice, used to extract textual information from the node.
    /// * `imports` - A reference to a `HashMap` mapping aliases to their fully qualified paths.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the callable name cannot be extracted
    /// or resolved, or if the node kind is unsupported.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Given a call like `commands::run_command()`
    /// let indexer = RustIndexer;
    /// let imports = &mut HashMap::new();
    /// // Assume imports are populated accordingly
    /// let callable_name = indexer.extract_callable_name(node, code, imports)?;
    /// assert_eq!(callable_name, "run_command");
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` containing the callable name as a `String` on success,
    /// or a `ContextMeshError` on failure.
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
    ///
    /// This method updates the `current_module` stack to reflect the nesting of modules.
    /// If the AST node represents a module declaration (`mod`), it extracts the module name
    /// and pushes it onto the `current_module` stack.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the module declaration.
    /// * `code` - The source code as a byte slice, used to extract the module name.
    /// * `current_module` - A mutable reference to a vector maintaining the stack of current modules.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the module name cannot be extracted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Given a module declaration: mod utils { ... }
    /// let indexer = RustIndexer;
    /// let node = /* AST node for 'mod utils { ... }' */;
    /// let code = b"mod utils { ... }";
    /// let mut current_module = Vec::new();
    /// indexer.enter_module(node, code, &mut current_module)?;
    /// assert_eq!(current_module, vec!["utils"]);
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the module was successfully entered,
    /// or a `ContextMeshError` if an error occurred.
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
    ///
    /// This method updates the `current_module` stack by popping the last module name,
    /// reflecting the exit from the current module scope.
    ///
    /// # Arguments
    ///
    /// * `current_module` - A mutable reference to a vector maintaining the stack of current modules.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the module stack cannot be updated correctly.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let indexer = RustIndexer;
    /// let mut current_module = vec!["utils".to_string()];
    /// indexer.exit_module(&mut current_module)?;
    /// assert!(current_module.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the module was successfully exited,
    /// or a `ContextMeshError` if an error occurred.
    fn exit_module(&self, current_module: &mut Vec<String>) -> Result<(), ContextMeshError> {
        if !current_module.is_empty() {
            current_module.pop();
        }
        Ok(())
    }
}
