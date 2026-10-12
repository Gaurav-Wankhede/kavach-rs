//! Error types for rule storage operations.

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("rule not found: {0}")]
    NotFound(String),

    #[error("TOON parse error: {0}")]
    ParseError(String),

    #[error("frontmatter error: {0}")]
    FrontmatterError(String),

    #[error("serialization error: {0}")]
    SerializeError(String),

    #[error("file lock failed: {0}")]
    LockFailed(String),

    #[error("atomic rename failed: {source}")]
    AtomicRename { source: std::io::Error },
}

pub type Result<T> = std::result::Result<T, StorageError>;
