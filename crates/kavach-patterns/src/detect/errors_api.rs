//! API client-specific error and implementation patterns.
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::is_api_client_file;
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

/// Detect API client issues (stale routes, hardcoded URLs, missing auth headers).
pub(super) fn detect_api_client_errors(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if !is_api_client_file(fp) {
        return;
    }
    ck(
        res,
        idx(r, 59),
        content,
        AntiProdLevel::P1ProdLeak,
        "API_DRIFT",
        "NOT_IMPLEMENTED comment",
        "Backend may be implemented — verify route exists and remove stale comment.",
    );
    if idx(r, 60).is_match(content) && !content.contains("localhost") {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "HARDCODED_URL",
            "hardcoded API base URL",
            "Use env var (import.meta.env.VITE_API_URL) — hardcoded URLs break env promotion.",
        );
    }
    ck(
        res,
        idx(r, 61),
        content,
        AntiProdLevel::P1ProdLeak,
        "EMPTY_FETCH",
        "async fn returns hardcoded empty",
        "Fetch from API — hardcoded [] or {} hides unimplemented routes.",
    );
    let has_token_var = content.contains("accessToken")
        || content.contains("authToken")
        || content.contains("bearerToken");
    let has_fetch = content.contains("fetch(") || content.contains("axios.");
    let has_auth_header = content.contains("Authorization") || content.contains("authorization");
    if has_token_var && has_fetch && !has_auth_header {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "AUTH_LEAK",
            "token defined but not forwarded",
            "Add Authorization header to fetch — token is in scope but not sent.",
        );
    }
}
