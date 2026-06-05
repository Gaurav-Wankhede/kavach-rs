//! Go language-specific anti-patterns.
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

/// Detect Go language-specific issues.
pub(super) fn detect_go_lang(res: &mut Vec<AntiProdResult>, r: &[Regex], fp: &str, content: &str) {
    if idx(r, 28).is_match(content) {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "fmt.Print",
            "Use structured logger.",
        );
    }
    if idx(r, 32).is_match(content) && fbase(fp) != "main.go" {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "os.Exit outside main",
            "Return error.",
        );
    }
}
