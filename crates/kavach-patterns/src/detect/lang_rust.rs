//! Rust language-specific anti-patterns.
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::regex_patterns::fbase;
use regex::Regex;

#[inline]
#[expect(
    clippy::indexing_slicing,
    reason = "index is a constant literal bounded by regex array size"
)]
fn idx(r: &[Regex], i: usize) -> &Regex {
    &r[i]
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

/// Detect Rust language-specific issues.
pub(super) fn detect_rust_lang(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if idx(r, 17).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "debug-macro",
            "Remove debug macro.",
        );
    }
    if idx(r, 18).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "print-macro",
            "Use tracing/log.",
        );
    }
    if idx(r, 19).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "stub-macro",
            "Implement first.",
        );
    }
    let b = fbase(fp);
    if idx(r, 21).is_match(content) && b != "main.rs" {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "rs-abort-macro",
            "Return Result.",
        );
    }
    if idx(r, 22).is_match(content) && b != "main.rs" {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "proc-exit",
            "Skips destructors.",
        );
    }
    if idx(r, 20).is_match(content) && !content.contains("// SAFETY:") {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "unsafe block",
            "Add // SAFETY:.",
        );
    }
}
