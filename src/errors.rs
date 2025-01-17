use std::fmt;
use std::io;

/// Represents all possible errors in the ContextMesh application.
#[derive(Debug)]
pub enum ContextMeshError {
    IoError(io::Error),
    SerdeError(bincode::Error),
    TreeSitterError(String),
    UnsupportedLanguage(String),
    SerializationError(String),
    DeserializationError(String),
    ClipboardError(String),
    IndexNotFound(String),
}

impl fmt::Display for ContextMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextMeshError::IoError(e) => write!(f, "IO Error: {}", e),
            ContextMeshError::SerdeError(e) => {
                write!(f, "Serialization/Deserialization Error: {}", e)
            }
            ContextMeshError::TreeSitterError(e) => write!(f, "Tree-sitter Parsing Error: {}", e),
            ContextMeshError::UnsupportedLanguage(lang) => {
                write!(f, "Unsupported language: {}", lang)
            }
            ContextMeshError::SerializationError(e) => write!(f, "Serialization Error: {}", e),
            ContextMeshError::DeserializationError(e) => write!(f, "Deserialization Error: {}", e),
            ContextMeshError::ClipboardError(e) => write!(f, "Clipboard Error: {}", e),
            ContextMeshError::IndexNotFound(path) => {
                write!(f, "Index file not found at path: {}", path)
            }
        }
    }
}

impl std::error::Error for ContextMeshError {}

impl From<io::Error> for ContextMeshError {
    fn from(error: io::Error) -> Self {
        ContextMeshError::IoError(error)
    }
}

impl From<bincode::Error> for ContextMeshError {
    fn from(error: bincode::Error) -> Self {
        ContextMeshError::SerdeError(error)
    }
}

impl From<tree_sitter::LanguageError> for ContextMeshError {
    fn from(_error: tree_sitter::LanguageError) -> Self {
        ContextMeshError::TreeSitterError("Failed to load language grammar.".to_string())
    }
}
