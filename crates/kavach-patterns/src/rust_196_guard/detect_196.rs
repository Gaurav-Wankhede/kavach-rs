//! New-idiom checks specific to Rust 1.96 (assert_matches!, double_negations, core::range Copy).

use super::patterns::get_pattern;
use super::types::{Rust196Severity, Rust196Violation};

/// Append Rust-1.96 idiom advisories for `content` to `violations`.
pub(super) fn check(content: &str, violations: &mut Vec<Rust196Violation>) {
    if get_pattern(19).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "assert!(matches!) over assert_matches!",
            fix: "Rust 1.96 stabilized assert_matches!: `assert_matches!(x, P)` gives a clearer panic than `assert!(matches!(x, P))`.",
            line: 0,
        });
    }
    if get_pattern(20).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "double negation",
            fix: "Rust 1.96 lints double_negations: `- -x` is a no-op (likely a typo'd `--x` prefix-decrement). Remove it or write `x`.",
            line: 0,
        });
    }
    if get_pattern(21).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P2Warning,
            pattern: "manual range struct over core::range",
            fix: "Rust 1.96 made core::range::Range Copy (RFC3550). Use `Range<usize>` to store a span in a Copy struct instead of hand-rolled start/end.",
            line: 0,
        });
    }
}
