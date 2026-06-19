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

/// True when `fp`/`content` is a TEST context. Handler patterns (`EMPTY_RESPONSE`,
/// `STUB_BODY`, …) describe production handlers; a test file or `#[cfg(test)]`
/// module legitimately contains empty/stub bodies and prose like "empty response
/// body" in doc comments — flagging them is a P0 false-positive that blocked
/// `#[ignore]`-gated DB/Scylla integration tests.
/// Ref: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
#[inline]
fn is_test_context(fp: &str, content: &str) -> bool {
    fp.contains("/tests/")
        || fp.ends_with("_test.rs")
        || fp.ends_with("tests.rs")
        || fp.contains("_test.")
        || content.contains("#[cfg(test)]")
        || content.contains("#[tokio::test]")
        || content.contains("#[test]")
}

/// True when `fp` is a BINARY/CLI/worker entrypoint, NOT an HTTP route handler.
/// `is_handler_file` treats EVERY `main.rs`/`lib.rs` as a handler, but a CLI tool,
/// migrator, or worker binary serves NO HTTP response — so `EMPTY_RESPONSE`
/// ("empty response hides an unimplemented handler") and the other web-route
/// patterns are P0 FALSE-POSITIVES there. A `fn main() -> Result<()>` MUST end in
/// `Ok(())`, which the `EMPTY_RESPONSE` regex matched, blocking every binary `main`.
/// Scope: `/tools/` + `/bin/` dirs + `main.rs` under a non-service crate.
/// Ref: operator directive 2026-06-18 (dbx migrator binary blocked by this FP).
#[inline]
fn is_binary_entrypoint(fp: &str) -> bool {
    let l = fp.to_lowercase();
    l.contains("/tools/")
        || l.contains("/bin/")
        || (l.ends_with("/main.rs")
            && !l.contains("/services/")
            && !l.contains("/api/"))
}

/// Detect handler-specific patterns (only fires on a production `is_handler_file`,
/// never in a test context).
pub(super) fn detect_handler_patterns(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if !is_handler_file(fp) || is_test_context(fp, content) || is_binary_entrypoint(fp) {
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
