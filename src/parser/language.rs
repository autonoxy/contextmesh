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
    fn language_name(&self) -> &'static str;

    /// Provides a list of node kinds that represent top-level definitions in the language.
    /// Top-level definitions include constructs like functions, classes, structs, enums, etc.,
    /// depending on the language's syntax.
    fn allowed_definition_kinds(&self) -> &'static [&'static str];

    /// Constructs the fully qualified name of a symbol given its AST node.
    fn build_qualified_name(&self, node: Node, code: &[u8]) -> Result<String, ContextMeshError>;

    /// Parses import or use declarations in the code to populate the `imports` map.
    fn process_import_declaration(
        &self,
        node: Node,
        code: &[u8],
        imports: &mut HashMap<String, String>,
    ) -> Result<(), ContextMeshError>;

    /// Extracts the name of a callable entity (e.g., function, method) from a reference node.
    fn extract_callable_name(
        &self,
        node: Node,
        code: &[u8],
        imports: &HashMap<String, String>,
    ) -> Result<String, ContextMeshError>;

    /// Handles entering a new module or namespace scope during parsing.
    fn enter_module(
        &self,
        node: Node,
        code: &[u8],
        current_module: &mut Vec<String>,
    ) -> Result<(), ContextMeshError>;

    /// Handles exiting a module or namespace scope during parsing.
    fn exit_module(&self, current_module: &mut Vec<String>) -> Result<(), ContextMeshError>;
}
