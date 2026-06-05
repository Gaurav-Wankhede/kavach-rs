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
mod scanner_category;
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

/// Scan a specific pattern category.
#[must_use]
pub fn scan_category(category: PatternCategory, content: &str) -> Vec<PatternMatch> {
    scanner_category::scan(category, content)
}

/// Count critical severity matches.
#[must_use]
pub fn count_critical(matches: &[PatternMatch]) -> usize {
    matches
        .iter()
        .filter(|m| m.severity == Severity::P0Critical)
        .count()
}

/// Check if any blocking violations exist (P0 or P1).
#[must_use]
pub fn has_blocking_violations(matches: &[PatternMatch]) -> bool {
    matches
        .iter()
        .any(|m| m.severity == Severity::P0Critical || m.severity == Severity::P1High)
}

#[cfg(test)]
#[path = "production_patterns_test.rs"]
mod tests;
