//! Dockerfile-specific anti-patterns.
use std::sync::OnceLock;

use crate::config::{AntiProdLevel, AntiProdResult};
use regex::Regex;

/// Fallback for an out-of-range static-table index: never matches. The tables
/// are compile-time constants, so this path is unreachable in practice; the
/// debug_assert fires in tests instead of panicking in production.
static NEVER_MATCH: OnceLock<Regex> = OnceLock::new();

#[inline]
fn idx_or_skip(r: &[Regex], i: usize) -> &Regex {
    r.get(i).unwrap_or_else(|| {
        debug_assert!(false, "regex table index {i} out of range");
        r.first().unwrap_or_else(|| {
            NEVER_MATCH.get_or_init(|| {
                #[expect(
                    clippy::expect_used,
                    reason = "static regex string is a compile-time constant"
                )]
                Regex::new("\\b\\B").expect("never-match fallback regex compiles")
            })
        })
    })
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

/// Detect Dockerfile-specific issues.
pub(super) fn detect_dockerfile_lang(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,
        idx_or_skip(r, 41),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "FROM :latest",
        "Pin version.",
    );
    if !idx_or_skip(r, 45).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "no USER directive",
            "Runs as root.",
        );
    }
}
