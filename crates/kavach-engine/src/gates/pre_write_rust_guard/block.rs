//! P0 hard-block: production-code violations (unwrap, `_var`, panic, as-cast).
use kavach_patterns::rust_guard::{RustSeverity, RustViolation};
use std::fmt::Write as _;

/// `Some(reason)` when any P0 violation is present (P1 advisories appended).
pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    let violations = kavach_patterns::rust_guard::detect(file_path, content);
    if violations.is_empty() {
        return None;
    }
    let p0: Vec<&RustViolation> = violations
        .iter()
        .filter(|v| v.severity == RustSeverity::P0Block)
        .collect();
    if p0.is_empty() {
        return None;
    }
    let p1: Vec<&RustViolation> = violations
        .iter()
        .filter(|v| v.severity == RustSeverity::P1Advisory)
        .collect();

    let mut msg = String::from(
        "[RUST_LAW] Production code violations detected\n\n\
         P0 VIOLATIONS:\n",
    );
    for v in &p0 {
        writeln!(msg, "  {} — {}", v.pattern, v.fix).ok();
    }
    if !p1.is_empty() {
        msg.push_str("\nP1 ADVISORIES:\n");
        for v in &p1 {
            writeln!(msg, "  {} — {}", v.pattern, v.fix).ok();
        }
    }
    msg.push_str(
        "\nREQUIRED: Fix all P0 violations before this write can proceed.\n\n\
         RESEARCH: WebSearch \"rust error handling best practices {search_year}\"\n\
         SKILL: Invoke `error` skill for propagation patterns.\n\
         FIX: Use `?` operator for propagation, `thiserror` for custom types,\n\
         `map_err` to add context. Never `unwrap`/`expect` in production.",
    );
    Some(msg)
}
