//! Handler-specific anti-patterns (for routes, controllers, endpoints).
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::is_handler_file;
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

/// Detect handler-specific patterns (only fires on `is_handler_file`).
pub(super) fn detect_handler_patterns(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if !is_handler_file(fp) {
        return;
    }
    ck(
        res,
        idx(r, 52),
        content,
        AntiProdLevel::P0MockData,
        "STUB_BODY",
        "stub response body",
        "Implement real response — no placeholder text.",
    );
    ck(
        res,
        idx(r, 53),
        content,
        AntiProdLevel::P0MockData,
        "STATUS_MISUSE",
        "500 for auth",
        "Use 401/403 not 500 for auth failures.",
    );
    ck(
        res,
        idx(r, 54),
        content,
        AntiProdLevel::P0MockData,
        "STATUS_MISUSE",
        "404 for forbidden",
        "Use 403 not 404 for authorization failures.",
    );
    ck(
        res,
        idx(r, 55),
        content,
        AntiProdLevel::P0MockData,
        "STATUS_MISUSE",
        "200+error body",
        "Return proper 4xx/5xx, not 200 with error JSON.",
    );
    ck(
        res,
        idx(r, 56),
        content,
        AntiProdLevel::P0MockData,
        "N_PLUS_1",
        "query inside loop",
        "Extract query before loop — N+1 causes O(n) DB round trips.",
    );
    ck(
        res,
        idx(r, 57),
        content,
        AntiProdLevel::P1ProdLeak,
        "NESTED_LOOP",
        "nested loop O(n²)",
        "Use HashMap lookup, index, or single-pass algorithm.",
    );
    ck(
        res,
        idx(r, 58),
        content,
        AntiProdLevel::P0MockData,
        "EMPTY_RESPONSE",
        "empty response body",
        "Return meaningful data — empty responses hide unimplemented handlers.",
    );
}
