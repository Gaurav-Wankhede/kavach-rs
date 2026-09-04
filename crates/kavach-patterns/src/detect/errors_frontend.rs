//! Frontend error handling anti-patterns.
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

/// Detect frontend error handling issues (silent catches, missing error handlers).
pub(super) fn detect_frontend_errors(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,        idx_or_skip(r, 4),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        ".catch(()=>{})",
        "Handle errors.",
    );
    ck(
        res,        idx_or_skip(r, 5),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "non-null (!.)",
        "Use ?..",
    );
    if idx_or_skip(r, 6).is_match(content) && !content.contains(".catch") && !content.contains("try") {
        add(
            res,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "fetch no error",
            "try/catch.",
        );
    }
}
