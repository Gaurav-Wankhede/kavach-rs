//! Frontend error handling anti-patterns.
use crate::config::{AntiProdLevel, AntiProdResult};
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

/// Detect frontend error handling issues (silent catches, missing error handlers).
pub(super) fn detect_frontend_errors(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,
        idx(r, 4),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        ".catch(()=>{})",
        "Handle errors.",
    );
    ck(
        res,
        idx(r, 5),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "non-null (!.)",
        "Use ?..",
    );
    if idx(r, 6).is_match(content) && !content.contains(".catch") && !content.contains("try") {
        add(
            res,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "fetch no error",
            "try/catch.",
        );
    }
}
