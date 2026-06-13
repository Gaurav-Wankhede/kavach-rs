//! Rust anti-pattern + production-pattern detector.
//!
//! Severity tiers live in the shared `crate::severity` module (single source of
//! truth); the detector arms are split into
//! `rust_guard/{line_scan,multiline_core,multiline_canon}.rs` leaves to honor the
//! ≤100-LOC micro-file law. `detect()` is the thin orchestrator that owns the
//! early-exit guards + the shared `RUST_P` table.

use crate::rust_patterns::RUST_P;

// Historical public names kept stable as aliases over the shared types so
// callers (`kavach-engine`) and the leaf detectors need no churn.
pub use crate::severity::Severity as RustSeverity;
pub use crate::severity::Violation as RustViolation;

mod env_var;
mod line_scan;
mod multiline_canon;
mod multiline_core;

/// Detect Rust anti-patterns in `content`. Returns every violation found, or an
/// empty vec for non-Rust / test / allowlisted files.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<RustViolation> {
    if !crate::file_types::is_rust_file(file_path) || content.is_empty() {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) || crate::file_types::is_allowlisted(file_path) {
        return vec![];
    }

    let r = &*RUST_P;
    let base = crate::regex_patterns::fbase(file_path);
    let has_serde_derive = content.contains("#[derive(") && content.contains("Deserialize");

    let mut violations = Vec::new();
    line_scan::scan(r, content, &base, has_serde_derive, &mut violations);
    multiline_core::scan(r, content, &mut violations);
    multiline_canon::scan(r, content, &mut violations);
    env_var::scan(file_path, content, &mut violations);
    violations
}

#[cfg(test)]
#[path = "rust_guard_test.rs"]
mod tests;
