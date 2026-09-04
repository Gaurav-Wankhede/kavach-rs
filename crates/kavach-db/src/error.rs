use thiserror::Error;

#[derive(Error, Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively in kavach-rpc surreal_to_rpc; non_exhaustive forces a catch-all that drops new-variant context"
)]
pub enum Error {
    #[error("SurrealDB error: {0}")]
    Surreal(#[from] surrealdb::Error),

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

/// True only for `SurrealDB`'s table-not-found error (`The table '<name>' does
/// not exist`), which a fresh graph raises on the first SELECT before any write.
///
/// Anchored to the `The table '` shape so it does NOT also swallow the sibling
/// `does not exist` errors (field, function, param, index, …) — masking one of
/// those as an empty result would hide a genuine malformed query. SOURCE:
/// surrealdb-core 3.1.4 `err` variants, each `The <kind> '<name>' does not exist`.
#[must_use]
pub(crate) fn is_missing_table_error(e: &surrealdb::Error) -> bool {
    let msg = e.to_string();
    msg.contains("The table '") && msg.contains("' does not exist")
}

#[cfg(test)]
#[path = "error_test.rs"]
mod error_test;
