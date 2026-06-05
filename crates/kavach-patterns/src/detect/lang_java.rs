//! Java language-specific anti-patterns.
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

/// Detect Java language-specific issues.
pub(super) fn detect_java_lang(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,
        idx(r, 38),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "System.out",
        "Use SLF4J.",
    );
}
