use std::collections::HashMap;
use tree_sitter::Node;

use crate::errors::ContextMeshError;

/// Defines how to parse a specific language's code (Rust, Python, etc.),
/// building "fully qualified" names and references.
pub trait LanguageIndexer {
    /// The name for this language, e.g., "rust", "python".
    fn language_name(&self) -> &'static str;

    /// Node kinds that represent top-level definitions (e.g., fn, class, struct).
    fn allowed_definition_kinds(&self) -> &'static [&'static str];

    /// Given a node for a definition, build its fully qualified name.
    fn build_qualified_name(&self, node: Node, code: &[u8]) -> Result<String, ContextMeshError>;

    /// If the language supports import/use statements, parse them to fill `imports`.
    fn process_import_declaration(
        &self,
        node: Node,
        code: &[u8],
        imports: &mut HashMap<String, String>,
    ) -> Result<(), ContextMeshError>;

    /// Given a node that represents a call/reference, return a string
    /// that might match a local definition's name.
    fn extract_callable_name(
        &self,
        node: Node,
        code: &[u8],
        imports: &HashMap<String, String>,
    ) -> Result<String, ContextMeshError>;

    /// Handle entering a new module scope.
    fn enter_module(
        &self,
        node: Node,
        code: &[u8],
        current_module: &mut Vec<String>,
    ) -> Result<(), ContextMeshError>;

    /// Handle exiting a module scope.
    fn exit_module(&self, current_module: &mut Vec<String>) -> Result<(), ContextMeshError>;
}
