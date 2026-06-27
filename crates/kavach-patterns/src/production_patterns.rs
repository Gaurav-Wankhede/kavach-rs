//! Production anti-patterns: multi-category code audit.

mod api_interaction;
mod business_logic;
mod data_validation;
mod database;
mod error_handling;
mod proxy;
mod rls;
mod scalability;
mod scanner_all;
mod scanners;
mod security;
mod system_design;
mod types;

pub use types::{PatternCategory, PatternMatch, Severity};

/// Scan file content, skipping test files. Returns sorted matches.
#[must_use]
pub fn scan(file_path: &str, content: &str) -> Vec<PatternMatch> {
    if file_path.contains("/test") || file_path.contains("_test.") {
        return Vec::new();
    }
    scanner_all::scan(content)
}

/// Count critical severity matches.
#[must_use]
pub fn count_critical(matches: &[PatternMatch]) -> usize {
    matches
        .iter()
        .filter(|m| m.severity == Severity::P0Critical)
        .count()
}

#[cfg(test)]
#[path = "production_patterns_test.rs"]
#[cfg(test)]
#[path = "production_patterns_test.rs"]
mod tests;