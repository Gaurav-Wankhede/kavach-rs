// bulk_manifest module entry — single-RCA-bound batch edit authority.
// Splits into 3 nano-files: types (pure), sql (constants), ops (async I/O).
// Re-exports the public surface so callers see one tidy path.
mod ops;
mod sql;
mod types;

#[cfg(test)]
mod tests;

pub use ops::{bump_conformance, close, create, get, list_active, mark_expired};
pub use types::{BulkManifest, ConformanceField, CreateParams, is_usable};
