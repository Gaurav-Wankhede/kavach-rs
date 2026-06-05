//! Rust lint enforcement guard for Rust 1.95+ / 2024 Edition.
//!
//! Enforces lints based on ecosystem-wide analysis from crates.io.
//! SOURCE: <https://gist.github.com/timClicks/54a5eb46ff633bfc15d403c0c9984e8b>
//! Dataset: 24K+ lint configurations from published crates.
//!
//! Severity: `P1Advisory` — nudges toward best practice, doesn't block.

use crate::file_types;

/// Top enforced lints from ecosystem analysis (forbid level — zero tolerance).
const ECOSYSTEM_FORBID: &[(&str, u32)] = &[
    ("unsafe_code", 2900),
    ("missing_docs", 228),
    ("future_incompatible", 94),
];

/// Top enforced lints from ecosystem analysis (deny level — security/quality).
/// Note: `missing_docs` and `unsafe_code` removed — already in `ECOSYSTEM_FORBID` (stricter).
const ECOSYSTEM_DENY: &[(&str, u32)] = &[
    ("warnings", 1418),
    ("missing_debug_implementations", 747),
    ("clippy::all", 636),
    ("rust_2018_idioms", 591),
    ("dead_code", 503),
    ("rustdoc::broken_intra_doc_links", 370),
    ("clippy::pedantic", 269),
    ("unused_must_use", 187),
    ("nonstandard_style", 173),
    ("clippy::unwrap_used", 169),
    ("unsafe_op_in_unsafe_fn", 160),
    ("unreachable_pub", 141),
    ("missing_copy_implementations", 132),
    ("unused_qualifications", 129),
    ("clippy::cargo", 98),
];

/// Security-critical lints (kavach addition).
const SECURITY_DENY: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::unwrap_in_result",
    "clippy::indexing_slicing",
    "clippy::arithmetic_side_effects",
    "clippy::dbg_macro",
    "clippy::print_stdout",
    "clippy::print_stderr",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::mem_forget",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LintSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LintViolation {
    pub severity: LintSeverity,
    pub lint: String,
    pub message: String,
    pub ecosystem_count: Option<u32>,
}

fn is_crate_root(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|base| base == "lib.rs" || base == "main.rs")
}

fn content_has_lint(content: &str, lint: &str, levels: &[&str]) -> bool {
    levels.iter().any(|level| {
        let pattern = format!("#![{level}({lint})]");
        let alt = format!("{level}({lint})");
        content.contains(&pattern) || content.contains(&alt)
    })
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<LintViolation> {
    if !file_types::is_rust_file(file_path)
        || content.is_empty()
        || !is_crate_root(file_path)
        || file_types::is_test_file(file_path)
        || file_types::is_allowlisted(file_path)
    {
        return vec![];
    }

    let mut violations = Vec::new();

    for (lint, count) in ECOSYSTEM_FORBID {
        if !content_has_lint(content, lint, &["forbid"]) {
            violations.push(LintViolation {
                severity: LintSeverity::P1Advisory,
                lint: (*lint).to_owned(),
                message: format!("Missing #![forbid({lint})]. {count} crates enforce this."),
                ecosystem_count: Some(*count),
            });
        }
    }

    for (lint, count) in ECOSYSTEM_DENY {
        if !content_has_lint(content, lint, &["deny", "forbid"]) {
            violations.push(LintViolation {
                severity: LintSeverity::P1Advisory,
                lint: (*lint).to_owned(),
                message: format!("Missing #![deny({lint})]. {count} crates enforce this."),
                ecosystem_count: Some(*count),
            });
        }
    }

    for lint in SECURITY_DENY {
        if !content_has_lint(content, lint, &["deny", "forbid"]) {
            violations.push(LintViolation {
                severity: LintSeverity::P1Advisory,
                lint: (*lint).to_owned(),
                message: format!("Missing #![deny({lint})]. Security-critical for production."),
                ecosystem_count: None,
            });
        }
    }

    violations
}

#[must_use]
pub fn generate_strict_lint_block() -> String {
    use std::fmt::Write;
    let mut block = String::new();
    block.push_str("// Strict lint configuration for Rust 1.95+ / 2024 Edition\n");
    block.push_str(
        "// SOURCE: https://gist.github.com/timClicks/54a5eb46ff633bfc15d403c0c9984e8b\n\n",
    );

    block.push_str("// Ecosystem forbid (zero tolerance)\n");
    for (lint, count) in ECOSYSTEM_FORBID {
        _ = writeln!(block, "#![forbid({lint})]  // {count} crates");
    }
    block.push('\n');

    block.push_str("// Ecosystem deny (quality)\n");
    for (lint, count) in ECOSYSTEM_DENY.iter().take(10) {
        _ = writeln!(block, "#![deny({lint})]  // {count} crates");
    }
    block.push('\n');

    block.push_str("// Security-critical (kavach)\n");
    for lint in SECURITY_DENY {
        _ = writeln!(block, "#![deny({lint})]");
    }

    block
}

#[must_use]
pub fn top_lints(n: usize) -> Vec<(&'static str, &'static str, u32)> {
    let cap = ECOSYSTEM_FORBID.len().saturating_add(ECOSYSTEM_DENY.len());
    let mut all: Vec<(&str, &str, u32)> = ECOSYSTEM_FORBID
        .iter()
        .map(|(lint, count)| (*lint, "forbid", *count))
        .chain(
            ECOSYSTEM_DENY
                .iter()
                .map(|(lint, count)| (*lint, "deny", *count)),
        )
        .collect();
    debug_assert_eq!(all.len(), cap, "pre-sized capacity must match collected");
    all.sort_by_key(|x| std::cmp::Reverse(x.2));
    all.truncate(n);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_forbid_unsafe() {
        let violations = detect("src/lib.rs", "pub fn main() {}");
        assert!(violations.iter().any(|v| v.lint == "unsafe_code"));
    }

    #[test]
    fn accepts_forbid_unsafe() {
        let violations = detect("src/lib.rs", "#![forbid(unsafe_code)]\npub fn main() {}");
        let has_unsafe = violations.iter().any(|v| v.lint == "unsafe_code");
        assert!(!has_unsafe);
    }

    #[test]
    fn skips_non_root_files() {
        let violations = detect("src/utils.rs", "pub fn helper() {}");
        assert!(violations.is_empty());
    }

    #[test]
    fn generates_lint_block() {
        let block = generate_strict_lint_block();
        assert!(block.contains("#![forbid(unsafe_code)]"));
        assert!(block.contains("#![deny(clippy::unwrap_used)]")); // in SECURITY_DENY
    }

    #[test]
    fn top_lints_sorted_by_count() {
        let top = top_lints(5);
        assert!(top.len() >= 2);
        assert!(top[0].2 >= top[1].2);
    }
}
