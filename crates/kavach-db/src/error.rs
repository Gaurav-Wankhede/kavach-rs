use thiserror::Error;

#[derive(Error, Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively in kavach-rpc surreal_to_rpc; non_exhaustive forces a catch-all that drops new-variant context"
)]
pub enum Error {
    #[error("Turso error: {0}")]
    Turso(#[from] crate::turso::TursoError),

    #[error("libsql error: {0}")]
    Libsql(#[from] libsql::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Record not found: {0}")]
    RecordNotFound(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Invalid hierarchy: {0}")]
    InvalidHierarchy(String),

    #[error("Schema violation: {0}")]
    SchemaViolation(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
#[path = "error_test.rs"]
mod error_test;
