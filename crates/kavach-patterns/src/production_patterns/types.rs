//! Shared types and helpers for production pattern detection.

use regex::Regex;
use std::sync::LazyLock;

/// Never-matching regex, compiled once. `\b\B` (word-boundary AND
/// non-word-boundary at the same position) is a valid pattern that can never
/// match, and unlike `(?!)` it is not flagged by clippy's `invalid_regex`.
static NEVER_MATCH: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\b\B").ok());

/// Compile a compile-time-constant pattern. On the (unreachable) malformed-const
/// case we fall back to the cached never-match rather than crash.
pub(super) fn mk(p: &str) -> Option<Regex> {
    Regex::new(p).ok().or_else(|| NEVER_MATCH.clone())
}

/// Drop rows whose pattern failed to compile, yielding the `(Regex, ...)`
/// tuples the scanner consumes — no unwrap needed.
pub(super) fn compiled(
    rows: Vec<(Option<Regex>, &'static str, &'static str, Severity)>,
) -> Vec<(Regex, &'static str, &'static str, Severity)> {
    rows.into_iter()
        .filter_map(|(r, c, m, s)| r.map(|r| (r, c, m, s)))
        .collect()
}

/// Production pattern categories with severity levels.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCategory {
    BusinessLogic,
    ErrorHandling,
    DataValidation,
    ApiInteraction,
    Security,
    Database,
    RowLevelSecurity,
    Proxy,
    Scalability,
    SystemDesign,
}

/// Pattern match result.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub category: PatternCategory,
    pub code: &'static str,
    pub message: &'static str,
    pub severity: Severity,
    pub line: usize,
    pub matched: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    P0Critical,
    P1High,
    P2Medium,
    P3Low,
}
