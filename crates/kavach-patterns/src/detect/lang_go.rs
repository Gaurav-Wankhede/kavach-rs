//! Go language-specific anti-patterns.
use std::sync::OnceLock;

use crate::config::{AntiProdLevel, AntiProdResult};
use crate::regex_patterns::fbase;
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
fn add(
    v: &mut Vec<AntiProdResult>,
    lv: AntiProdLevel,
    code: &'static str,
    mt: &str,
    msg: &'static str,
) {
    v.push(AntiProdResult {
        level: lv,
        code,
        match_text: mt.into(),
        message: msg,
    });
}

/// Detect Go language-specific issues.
pub(super) fn detect_go_lang(res: &mut Vec<AntiProdResult>, r: &[Regex], fp: &str, content: &str) {
    if idx_or_skip(r, 28).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "fmt.Print",
            "Use structured logger.",
        );
    }
    if idx_or_skip(r, 32).is_match(content) && fbase(fp) != "main.go" {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "os.Exit outside main",
            "Return error.",
        );
    }
}
