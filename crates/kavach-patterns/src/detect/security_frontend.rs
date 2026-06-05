//! Frontend security anti-patterns detector.
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::has_env_fallback;
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

/// Detect frontend security issues (console output, code execution, unsafe DOM).
pub(super) fn detect_frontend_security(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,
        idx(r, 0),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "console.log",
        "Remove debug output.",
    );
    for line in content.lines() {
        if idx(r, 3).is_match(line) && !has_env_fallback(line) {
            add(
                res,
                AntiProdLevel::P1ProdLeak,
                "PROD_LEAK",
                "proc.env no fallback",
                "Add ?? fallback.",
            );
            break;
        }
    }
    ck(
        res,
        idx(r, 13),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "eval()",
        "Code injection.",
    );
    ck(
        res,
        idx(r, 14),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "new Function()",
        "eval equiv.",
    );
    ck(
        res,
        idx(r, 15),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "document.write",
        "XSS risk.",
    );
    ck(
        res,
        idx(r, 11),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "dangerousSetHTML",
        "XSS risk.",
    );
    ck(
        res,
        idx(r, 12),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "unsafe DOM assign",
        "XSS risk.",
    );
}
