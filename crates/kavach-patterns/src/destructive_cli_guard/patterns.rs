//! Compiled regex pattern table for the destructive-CLI guard (leaf hub).
//!
//! Rows are grouped into two leaves to stay within the micro-file budget:
//! `destructive` (classic destructive shell ops) and `codeexec` (allowlisted-by-
//! name tools weaponized via a code-exec / file-write flag — the FN class).
use super::{DestructiveCategory, DestructiveSeverity};
use regex::Regex;
use std::sync::LazyLock;

mod codeexec;
mod destructive;

pub(super) type PatternRow = (
    DestructiveCategory,
    DestructiveSeverity,
    &'static str,
    &'static str,
    Regex,
);

/// `(category, severity, name, fix, regex-source)` tuple compiled by [`mk`].
type RawRow = (
    DestructiveCategory,
    DestructiveSeverity,
    &'static str,
    &'static str,
    &'static str,
);

/// Compile one row's regex. PANICS on a bad pattern: these are hardcoded `const`
/// security rules, so a failed compile is a programmer error, never runtime input.
/// Failing loud here is mandatory — a silent `.ok()` drop would disable a P0 gate
/// (e.g. the `rg --pre` RCE block) with no error, no log, no test catch (fail-open
/// on a security boundary). `expect` runs once at first `PATTERNS` access.
// `re_str` is ALWAYS a hardcoded `const` from the ROWS tables — never runtime
// input — so a compile failure is a programmer typo. A silent `.ok()` drop (as
// sibling tables do) would disable a P0 security rule (e.g. the `rg --pre` RCE
// block) with no error, no log, no test catch — fail-OPEN on a security boundary.
#[expect(
    clippy::panic,
    reason = "const security regex: a bad compile MUST fail loud, never silently drop a P0 rule"
)]
fn mk(row: RawRow) -> PatternRow {
    let (cat, sev, pat, fix, re_str) = row;
    let re = Regex::new(re_str).unwrap_or_else(|e| {
        panic!("destructive_cli_guard: regex for `{pat}` failed to compile: {e}")
    });
    (cat, sev, pat, fix, re)
}

pub(super) static PATTERNS: LazyLock<Vec<PatternRow>> = LazyLock::new(|| {
    destructive::ROWS
        .iter()
        .chain(codeexec::ROWS.iter())
        .copied()
        .map(mk)
        .collect()
});
