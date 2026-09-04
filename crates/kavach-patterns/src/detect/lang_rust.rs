//! Rust language-specific anti-patterns.
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

/// True when `fp`/`content` is a Rust TEST context, where production-leak
/// patterns (empty fn bodies, proc-exit) are legitimate (mock impls, fixtures,
/// `#[ignore]`-gated integration tests). Prevents the `EMPTY_RESPONSE` / `proc-exit`
/// P0 false-positive that blocked legit DB/Scylla integration tests.
/// Ref: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
#[inline]
fn is_test_context(fp: &str, content: &str) -> bool {
    fp.contains("/tests/")
        || fp.ends_with("_test.rs")
        || fp.ends_with("tests.rs")
        || content.contains("#[cfg(test)]")
        || content.contains("#[tokio::test]")
        || content.contains("#[test]")
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
    if idx_or_skip(r, 17).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "debug-macro",
            "Remove debug macro.",
        );
    }
    if idx_or_skip(r, 18).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "print-macro",
            "Use tracing/log.",
        );
    }
    if idx_or_skip(r, 19).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "stub-macro",
            "Implement first.",
        );
    }
    let b = fbase(fp);
    let test_ctx = is_test_context(fp, content);
    if idx_or_skip(r, 21).is_match(content) && b != "main.rs" && !test_ctx {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "rs-abort-macro",
            "Return Result.",
        );
    }
    if idx_or_skip(r, 22).is_match(content) && b != "main.rs" && !test_ctx {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "proc-exit",
            "Skips destructors.",
        );
    }
    if idx_or_skip(r, 20).is_match(content) && !content.contains("// SAFETY:") {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "unsafe block",
            "Add // SAFETY:.",
        );
    }
}
