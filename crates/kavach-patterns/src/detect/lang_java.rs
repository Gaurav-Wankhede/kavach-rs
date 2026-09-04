//! Java language-specific anti-patterns.
use std::sync::OnceLock;

use crate::config::{AntiProdLevel, AntiProdResult};
use regex::Regex;

/// Fallback for an out-of-range static-table index: never matches. The tables
/// are compile-time constants, so this path is unreachable in practice; the
/// debug_assert fires in tests instead of panicking in production.
static NEVER_MATCH: OnceLock<Regex> = OnceLock::new();

#[inline]
fn idx_or_skip(r: &[Regex], i: usize) -> &Regex {
    match r.get(i) {
        Some(re) => re,
        None => {
            debug_assert!(false, "regex table index {i} out of range");
            r.first().unwrap_or(NEVER_MATCH.get_or_init(|| {
                Regex::new("$^").expect("trivial regex compiles")
            }))
        }
    }
}

#[inline]
fn ck(
    v: &mut Vec<AntiProdResult>,
    re: &Regex,
    c: &str,
    lv: AntiProdLevel,
    code: &'static str,
    mt: &str,
    msg: &'static str,
) {
    if re.is_match(c) {
        v.push(AntiProdResult {
            level: lv,
            code,
            match_text: mt.into(),
            message: msg,
        });
    }
}

/// Detect Java language-specific issues.
pub(super) fn detect_java_lang(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,        idx_or_skip(r, 38),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "System.out",
        "Use SLF4J.",
    );
}
