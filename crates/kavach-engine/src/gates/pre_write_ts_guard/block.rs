//! P0 hard-block + P1 advisory rendering of `ts_guard` violations.
use std::fmt::Write as _;

use kavach_patterns::ts_guard::{TsSeverity, TsViolation};

/// P0 frontend production violations -> a hard-block message (P1s appended for
/// context). Returns None when there are no P0 violations.
pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    let violations = kavach_patterns::ts_guard::detect(file_path, content);
    let p0: Vec<&TsViolation> = violations
        .iter()
        .filter(|v| v.severity == TsSeverity::P0Block)
        .collect();
    if p0.is_empty() {
        return None;
    }
    let p1: Vec<&TsViolation> = violations
        .iter()
        .filter(|v| v.severity == TsSeverity::P1Advisory)
        .collect();
    let mut msg = String::from(
        "[TS_LAW] Frontend production violations detected\n\n\
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
        "\n-> fix all P0 violations before this write can proceed.\n\n\
         RESEARCH: WebSearch \"typescript type safety best practices {search_year}\"\n\
         SKILL: Invoke `interface-design` skill for frontend patterns.\n\
         FIX: Fetch data from API. Replace `as any` with explicit types.\n\
         Use Zod/io-ts for runtime validation at API boundaries -> retry.",
    );
    Some(msg)
}

/// P1-only advisory block (no P0 gating). Returns None when there are no P1s.
pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    let violations = kavach_patterns::ts_guard::detect(file_path, content);
    let p1: Vec<&TsViolation> = violations
        .iter()
        .filter(|v| v.severity == TsSeverity::P1Advisory)
        .collect();
    if p1.is_empty() {
        return None;
    }
    let mut msg = String::from("[TS_GUARD_ADVISORY]\n");
    for v in &p1 {
        writeln!(msg, "  {} — {}", v.pattern, v.fix).ok();
    }
    Some(msg)
}
