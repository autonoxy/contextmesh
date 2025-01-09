use std::collections::HashMap;
use tree_sitter::Node;

use crate::errors::ContextMeshError;

/// Defines how to parse a specific programming language's code (e.g., Rust, Python),
/// constructing "fully qualified" names and references for symbols within the codebase.
///
/// Implementing this trait allows the parser to understand the syntax and semantics
/// of different languages, facilitating accurate symbol indexing and dependency mapping.
pub trait LanguageIndexer {
    /// Returns the name of the language that this indexer handles.
    ///
    /// # Examples
    ///
    /// ```rust
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     fn language_name(&self) -> &'static str {
    ///         "rust"
    ///     }
    ///     
    ///     // Other method implementations...
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A string slice representing the language name (e.g., "rust", "python").
    fn language_name(&self) -> &'static str;

    /// Provides a list of node kinds that represent top-level definitions in the language.
    ///
    /// Top-level definitions include constructs like functions, classes, structs, enums, etc.,
    /// depending on the language's syntax.
    ///
    /// # Examples
    ///
    /// ```rust
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     fn allowed_definition_kinds(&self) -> &'static [&'static str] {
    ///         &["function_item", "struct_item", "enum_item", "trait_item", "impl_item"]
    ///     }
    ///     
    ///     // Other method implementations...
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A slice of string slices, each representing a node kind that corresponds to a top-level definition.
    fn allowed_definition_kinds(&self) -> &'static [&'static str];

    /// Constructs the fully qualified name of a symbol given its AST node.
    ///
    /// This method extracts the symbol's name from the AST node and, if necessary,
    /// prepends module or namespace qualifiers to form a fully qualified name.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the symbol definition.
    /// * `code` - The source code as a byte slice, used to extract textual information from the node.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the name cannot be extracted or constructed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     // Other method implementations...
    ///
    ///     fn build_qualified_name(&self, node: Node, code: &[u8]) -> Result<String, ContextMeshError> {
    ///         // Implementation details...
    ///         Ok("fully::qualified::Name".to_string())
    ///     }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` containing the fully qualified name of the symbol as a `String` on success,
    /// or a `ContextMeshError` on failure.
    fn build_qualified_name(&self, node: Node, code: &[u8]) -> Result<String, ContextMeshError>;

    /// Parses import or use declarations in the code to populate the `imports` map.
    ///
    /// This method processes nodes that represent import statements (e.g., `use` in Rust,
    /// `import` in Python) and maps aliases to their fully qualified paths, facilitating
    /// the resolution of symbols that use these imports.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the import declaration.
    /// * `code` - The source code as a byte slice, used to extract textual information from the node.
    /// * `imports` - A mutable reference to a `HashMap` where aliases are mapped to their fully qualified paths.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the import cannot be processed or parsed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    ///
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     // Other method implementations...
    ///
    ///     fn process_import_declaration(
    ///         &self,
    ///         node: Node,
    ///         code: &[u8],
    ///         imports: &mut HashMap<String, String>,
    ///     ) -> Result<(), ContextMeshError> {
    ///         // Implementation details...
    ///         Ok(())
    ///     }
    /// }
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
    ) -> Result<(), ContextMeshError>;

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
    /// Returns `ContextMeshError::DeserializationError` if the callable name cannot be extracted or resolved.
    ///
    /// # Examples
    ///
    /// ```rust
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     // Other method implementations...
    ///
    ///     fn extract_callable_name(
    ///         &self,
    ///         node: Node,
    ///         code: &[u8],
    ///         imports: &HashMap<String, String>,
    ///     ) -> Result<String, ContextMeshError> {
    ///         // Implementation details...
    ///         Ok("fully::qualified::callable_name".to_string())
    ///     }
    /// }
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
    ) -> Result<String, ContextMeshError>;

    /// Handles entering a new module or namespace scope during parsing.
    ///
    /// This method updates the `current_module` stack to reflect the nesting of modules or namespaces.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node representing the module declaration.
    /// * `code` - The source code as a byte slice, used to extract textual information from the node.
    /// * `current_module` - A mutable reference to a vector maintaining the stack of current modules.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::DeserializationError` if the module name cannot be extracted or processed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     // Other method implementations...
    ///
    ///     fn enter_module(
    ///         &self,
    ///         node: Node,
    ///         code: &[u8],
    ///         current_module: &mut Vec<String>,
    ///     ) -> Result<(), ContextMeshError> {
    ///         // Implementation details...
    ///         Ok(())
    ///     }
    /// }
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
    ) -> Result<(), ContextMeshError>;

    /// Handles exiting a module or namespace scope during parsing.
    ///
    /// This method updates the `current_module` stack to reflect the exit from the current module or namespace.
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
    /// struct RustIndexer;
    ///
    /// impl LanguageIndexer for RustIndexer {
    ///     // Other method implementations...
    ///
    ///     fn exit_module(&self, current_module: &mut Vec<String>) -> Result<(), ContextMeshError> {
    ///         // Implementation details...
    ///         Ok(())
    ///     }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the module was successfully exited,
    /// or a `ContextMeshError` if an error occurred.
    fn exit_module(&self, current_module: &mut Vec<String>) -> Result<(), ContextMeshError>;
}
