//! P1/P2 advisories: production-pattern nudges + missing ecosystem lints.
use kavach_patterns::rust_guard::{RustSeverity, RustViolation};
use kavach_patterns::rust_lint_guard;
use std::fmt::Write as _;

/// P1 production-pattern advisory (clone, catch-all, allow), or None if clean.
pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    let violations = kavach_patterns::rust_guard::detect(file_path, content);
    let p1: Vec<&RustViolation> = violations
        .iter()
        .filter(|v| v.severity == RustSeverity::P1Advisory)
        .collect();
    if p1.is_empty() {
        return None;
    }
    let mut msg = String::from("[RUST_GUARD_ADVISORY]\n");
    for v in &p1 {
        writeln!(msg, "  {} — {}", v.pattern, v.fix).ok();
    }
    Some(msg)
}

/// Lint-enforcement advisory for crate roots (`lib.rs`/`main.rs`).
/// SOURCE: <https://gist.github.com/timClicks/54a5eb46ff633bfc15d403c0c9984e8b>
pub(crate) fn format_lint_advisory(file_path: &str, content: &str) -> Option<String> {
    let violations = rust_lint_guard::detect(file_path, content);
    let p1: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == rust_lint_guard::LintSeverity::P1Advisory)
        .collect();
    let p2: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == rust_lint_guard::LintSeverity::P2Warning)
        .collect();
    if p1.is_empty() && p2.is_empty() {
        return None;
    }
    let mut msg = String::from("[RUST_LINT_ADVISORY] Missing ecosystem-standard lints\n");
    msg.push_str("SOURCE: https://gist.github.com/timClicks/54a5eb46ff633bfc15d403c0c9984e8b\n\n");
    append_lint_section(&mut msg, &p1, &p2);
    msg.push_str("\nFIX: Run `kavach lint-block` to generate copy-paste lint configuration.\n");
    Some(msg)
}

/// Append the P1 (required) and P2 (recommended) lint sections.
fn append_lint_section(
    msg: &mut String,
    p1: &[&rust_lint_guard::LintViolation],
    p2: &[&rust_lint_guard::LintViolation],
) {
    if !p1.is_empty() {
        msg.push_str("REQUIRED (P1):\n");
        for v in p1.iter().take(5) {
            match v.ecosystem_count {
                Some(count) => {
                    writeln!(msg, "  #![deny/forbid({})]: {count} crates enforce", v.lint)
                }
                None => writeln!(msg, "  #![deny({})]: security-critical", v.lint),
            }
            .ok();
        }
        if p1.len() > 5 {
            writeln!(msg, "  ... and {} more", p1.len().saturating_sub(5)).ok();
        }
    }
    if !p2.is_empty() && p2.len() <= 5 {
        msg.push_str("\nRECOMMENDED (P2):\n");
        for v in p2.iter().take(3) {
            if let Some(count) = v.ecosystem_count {
                writeln!(msg, "  #![deny({})]: {count} crates", v.lint).ok();
            }
        }
    }
}
