// split: pattern detection module — test strings contain async fn signatures
//! API Gateway pattern enforcement — detects missing gateway layer,
//! protocol leakage, and missing aggregation in handler/route files.
//!
//! Uses string-based pattern matching (similar to Semgrep approach) rather than
//! full AST parsing for speed. Patterns are language-agnostic where possible.

mod checks;
#[cfg(test)]
mod tests;

pub use checks::{Severity, Violation, ViolationKind, detect, is_handler_file};
