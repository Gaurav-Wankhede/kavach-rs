// SOURCE: https://docs.rs/thiserror/2 — derive macro for Error trait

use thiserror::Error;

#[derive(Debug, Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub enum DTreeError {
    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("invalid predicate: {0}")]
    InvalidPredicate(String),

    #[error("tree serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("feature missing: {0}")]
    FeatureMissing(String),
}
