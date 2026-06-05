use thiserror::Error;

/// Errors returned by the kavach-rag crate.
///
/// Every variant is recoverable by the caller: hook gates swallow and log,
/// CLI builders surface to the user. `#[non_exhaustive]` keeps future
/// additions non-breaking.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RagError {
    #[error("JSON parse failed: {0}")]
    Parse(#[source] serde_json::Error),

    #[error("tree validation failed: {0}")]
    Invalid(String),

    #[error("node id '{0}' not found in tree")]
    NodeNotFound(String),
}

impl From<serde_json::Error> for RagError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value)
    }
}
