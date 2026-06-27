// split: Single-module DSA gate. Test fixtures intentionally embed anti-pattern Rust source.
//
//   {"name":"syn AST walk","reason":"3-5x slower for write-time gate; needs full parse"},
//   {"name":"tree-sitter","reason":"adds 2MB binary, overkill for ~16 regex matches"},
//   {"name":"hand-rolled scanner","reason":"reinvents regex DFA poorly"}
// ]
// TIME: O(n) per file (single-pass NFA via regex crate) | SPACE: O(patterns)
// YEAR: 2026 | SEARCHED: 2026-05
//! DSA Gate — Data Structures & Algorithms for Rust Backends
//!
//! Detects accidental O(n) / O(n^2) / quadratic-allocation patterns that scale poorly under load.
//!
//! SOURCES (verified 2026-05):
//! - <https://doc.rust-lang.org/std/collections/index.html>
//! - <https://lib.rs/data-structures>
//! - <https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.entry>
//! - <https://docs.rs/rustc-hash/latest/rustc_hash>/

mod dispatch;
mod patterns;
mod types;

pub use dispatch::{detect, warn_count};
pub use types::{DsaClass, DsaSeverity, DsaViolation};

#[cfg(test)]
#[path = "dsa_guard_test.rs"]
mod tests;
