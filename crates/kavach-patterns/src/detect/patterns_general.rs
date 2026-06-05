//! General anti-patterns for all languages.
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::{
    is_dockerfile, is_marker_inside_string, is_non_config_file, is_shell_file,
};
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

/// Detect general cross-language patterns.
pub(super) fn detect_general_patterns(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    let has_bare_marker = content
        .lines()
        .any(|line| idx(r, 1).is_match(line) && !is_marker_inside_string(line));
    if has_bare_marker {
        add(
            res,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "task-marker",
            "Implement or create ticket.",
        );
    }
    if is_non_config_file(fp) {
        ck(
            res,
            idx(r, 2),
            content,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "localhost",
            "Use config var.",
        );
    }
    ck(
        res,
        idx(r, 51),
        content,
        AntiProdLevel::P0MockData,
        "STUB_501",
        "501 NOT_IMPLEMENTED",
        "Implement handler NOW — no 501 stubs.",
    );
    ck(
        res,
        idx(r, 42),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "broad-perms",
        "Use least-privilege.",
    );
    if is_dockerfile(fp) || is_shell_file(fp) {
        ck(
            res,
            idx(r, 43),
            content,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "curl|bash",
            "Verify first.",
        );
    }
}
