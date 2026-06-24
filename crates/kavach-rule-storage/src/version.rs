//! Version tracking and content-hash based change detection.

use std::fs;

use crate::error::{Result, StorageError};
use crate::store::StoredRule;

#[derive(Debug)]
#[non_exhaustive]
pub struct RuleVersion;

impl RuleVersion {
    /// Compute a BLAKE3 hex digest of the given content string.
    /// BLAKE3 replaces SHA-256 — 4x faster, same 128-bit collision resistance.
    #[must_use]
    pub fn compute_hash(content: &str) -> String {
        blake3::hash(content.as_bytes()).to_hex().to_string()
    }

    /// Check if the on-disk file differs from the stored content hash.
    ///
    /// # Errors
    /// Returns [`StorageError`] if the rule file is missing or cannot be read.
    pub fn has_file_changed(rule: &StoredRule) -> Result<bool> {
        let path = &rule.source_path;
        if !path.exists() {
            return Err(StorageError::NotFound(path.display().to_string()));
        }
        let content = fs::read_to_string(path)?;
        let current_hash = Self::compute_hash(&content);
        Ok(current_hash != rule.content_hash)
    }

}
