pub mod language; // The trait
pub mod rust_indexer; // The Rust plugin

use crate::{errors::ContextMeshError, indexer::symbol::Symbol};
use language::LanguageIndexer;
use rust_indexer::RustIndexer;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

/// `CodeParser` is responsible for parsing source files, extracting symbols,
/// and managing dependencies using a language-specific indexer.
///
/// It leverages the Tree-sitter parser to navigate the Abstract Syntax Tree (AST)
/// of source files and utilizes implementations of the `LanguageIndexer` trait
/// to handle language-specific parsing logic.
pub struct CodeParser {
    /// The Tree-sitter parser used to parse source code into an AST.
    parser: Parser,

    /// A boxed trait object implementing `LanguageIndexer`, allowing for
    /// language-specific parsing strategies (e.g., Rust, Python).
    plugin: Box<dyn LanguageIndexer>, // Language-specific implementation
}

impl CodeParser {
    /// Creates a new `CodeParser` instance configured for parsing Rust source files.
    ///
    /// This method initializes the Tree-sitter parser with the Rust language
    /// and sets up the Rust-specific indexer.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::TreeSitterError` if the parser fails to set the Rust language.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::errors::ContextMeshError;
    /// use crate::parser::CodeParser;
    ///
    /// let code_parser = CodeParser::new_rust().expect("Failed to create Rust CodeParser");
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` containing the initialized `CodeParser` on success,
    /// or a `ContextMeshError` on failure.
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

    /// Parses a single source file, extracting symbols and imports.
    ///
    /// This method performs the following steps:
    /// 1. Reads the source file from the provided `file_path`.
    /// 2. Parses the source code into an AST using Tree-sitter.
    /// 3. Traverses the AST to collect symbol definitions and import declarations.
    /// 4. Gathers references to these symbols to establish dependencies.
    ///
    /// # Arguments
    ///
    /// * `file_path` - A string slice representing the path to the source file to be parsed.
    ///
    /// # Errors
    ///
    /// Returns `ContextMeshError::IoError` if the file cannot be read.
    /// Returns `ContextMeshError::TreeSitterError` if parsing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::errors::ContextMeshError;
    /// use crate::parser::CodeParser;
    /// use std::collections::HashMap;
    ///
    /// let mut code_parser = CodeParser::new_rust()?;
    /// let (symbols, imports) = code_parser.parse_file("./src/main.rs")?;
    /// ```
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple with:
    /// - `Vec<Symbol>`: A vector of symbols extracted from the file.
    /// - `HashMap<String, String>`: A hashmap mapping aliases to their fully qualified paths.
    ///
    /// Or a `ContextMeshError` if an error occurs during parsing.
    pub fn parse_file(
        &mut self,
        file_path: &str,
    ) -> Result<(Vec<Symbol>, HashMap<String, String>), ContextMeshError> {
        println!(
            "Parsing file '{}' using {} indexer...",
            file_path,
            self.plugin.language_name()
        );

        // Read the source file into a byte vector
        let code = std::fs::read(file_path).map_err(|e| {
            eprintln!("Failed to read file {}: {}", file_path, e);
            ContextMeshError::IoError(e)
        })?;

        // Parse the source code into an AST
        let tree = self.parser.parse(&code, None).ok_or_else(|| {
            eprintln!("Failed to parse file {}.", file_path);
            ContextMeshError::TreeSitterError("Parsing returned no tree.".to_string())
        })?;

        let root = tree.root_node();

        let mut symbols = Vec::new();
        let mut imports = HashMap::new();

        // Initialize module stack to keep track of nested modules
        let mut current_module = Vec::new();

        // 1) Collect definitions and imports in one pass
        collect_definitions_and_imports(
            &*self.plugin,
            root,
            &code,
            file_path,
            &mut symbols,
            &mut imports,
            &mut current_module,
        )?;

        // 2) Gather references to establish dependencies
        let mut symbol_stack = Vec::new();
        gather_references(
            &*self.plugin,
            root,
            &code,
            file_path,
            &mut symbols,
            &imports,
            &mut symbol_stack,
        )?;

        Ok((symbols, imports))
    }
}

/// Traverses the AST to collect symbol definitions and import declarations.
///
/// This helper function performs a depth-first traversal of the AST node tree,
/// extracting symbols and imports using the provided `LanguageIndexer`.
///
/// # Arguments
///
/// * `lang` - A reference to an object implementing the `LanguageIndexer` trait.
/// * `node` - The current AST node being traversed.
/// * `code` - The source code as a byte slice, used to extract textual information from nodes.
/// * `file_path` - A string slice representing the path to the source file being parsed.
/// * `symbols` - A mutable reference to a vector where extracted symbols are stored.
/// * `imports` - A mutable reference to a hashmap where import declarations are stored.
/// * `current_module` - A mutable reference to a vector maintaining the stack of current modules.
///
/// # Errors
///
/// Returns `ContextMeshError::DeserializationError` if any part of the traversal fails.
fn collect_definitions_and_imports(
    lang: &dyn LanguageIndexer,
    node: Node,
    code: &[u8],
    file_path: &str,
    symbols: &mut Vec<Symbol>,
    imports: &mut HashMap<String, String>,
    current_module: &mut Vec<String>,
) -> Result<(), ContextMeshError> {
    // Enter module scope if the current node represents a module
    lang.enter_module(node, code, current_module)?;

    let node_kind = node.kind();

    // If the node is an import declaration, process it
    lang.process_import_declaration(node, code, imports)?;

    // If the node kind is among the allowed definitions, build and store the symbol
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

    // Recursively traverse all child nodes
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

    // Exit module scope if applicable
    lang.exit_module(current_module)?;

    Ok(())
}

/// Traverses the AST to gather references to previously collected symbols.
///
/// This helper function performs a depth-first traversal of the AST node tree,
/// identifying references to symbols and establishing dependencies between them.
///
/// # Arguments
///
/// * `lang` - A reference to an object implementing the `LanguageIndexer` trait.
/// * `node` - The current AST node being traversed.
/// * `code` - The source code as a byte slice, used to extract textual information from nodes.
/// * `file_path` - A string slice representing the path to the source file being parsed.
/// * `symbols` - A mutable reference to a vector where symbols are stored.
/// * `imports` - A reference to a hashmap containing import declarations.
/// * `symbol_stack` - A mutable reference to a vector maintaining the stack of current symbols.
///
/// # Errors
///
/// Returns `ContextMeshError::DeserializationError` if any part of the traversal fails.
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

    // If the node has a 'name' field, it might represent a new symbol scope
    if let Some(name_node) = node.child_by_field_name("name") {
        let start = name_node.start_position();
        if let Some((idx, _sym)) = symbols.iter().enumerate().find(|(_, s)| {
            s.file_path == file_path && s.line_number == start.row + 1 && s.node_kind == node_kind
        }) {
            symbol_stack.push(idx);

            // Recursively traverse child nodes within the new symbol scope
            for child in node.children(&mut node.walk()) {
                gather_references(lang, child, code, file_path, symbols, imports, symbol_stack)?;
            }

            symbol_stack.pop();
            return Ok(());
        }
    }

    // Handle function call expressions
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
    // Handle method call expressions (e.g., foo.bar(...))
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

    // Recursively traverse all child nodes
    for child in node.children(&mut node.walk()) {
        gather_references(lang, child, code, file_path, symbols, imports, symbol_stack)?;
    }

    Ok(())
}
