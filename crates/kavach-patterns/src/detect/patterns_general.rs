//! General anti-patterns for all languages.
use std::sync::OnceLock;

use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::{
    is_dockerfile, is_marker_inside_string, is_non_config_file, is_shell_file,
};
use regex::Regex;

/// Fallback for an out-of-range static-table index: never matches. The tables
/// are compile-time constants, so this path is unreachable in practice; the
/// debug_assert fires in tests instead of panicking in production.
static NEVER_MATCH: OnceLock<Regex> = OnceLock::new();

#[inline]
fn idx_or_skip(r: &[Regex], i: usize) -> &Regex {
    r.get(i).unwrap_or_else(|| {
        debug_assert!(false, "regex table index {i} out of range");
        r.first().unwrap_or_else(|| {
            NEVER_MATCH.get_or_init(|| {
                #[expect(
                    clippy::expect_used,
                    reason = "static regex string is a compile-time constant"
                )]
                Regex::new("\\b\\B").expect("never-match fallback regex compiles")
            })
        })
    })
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
        .any(|line| idx_or_skip(r, 1).is_match(line) && !is_marker_inside_string(line));
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
            idx_or_skip(r, 2),
            content,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "localhost",
            "Use config var.",
        );
    }
    ck(
        res,
        idx_or_skip(r, 51),
        content,
        AntiProdLevel::P0MockData,
        "STUB_501",
        "501 NOT_IMPLEMENTED",
        "Implement handler NOW — no 501 stubs.",
    );
    ck(
        res,
        idx_or_skip(r, 42),
        content,
        AntiProdLevel::P1ProdLeak,
        "PROD_LEAK",
        "broad-perms",
        "Use least-privilege.",
    );
    if is_dockerfile(fp) || is_shell_file(fp) {
        ck(
            res,
            idx_or_skip(r, 43),
            content,
            AntiProdLevel::P1ProdLeak,
            "PROD_LEAK",
            "curl|bash",
            "Verify first.",
        );
    }
}
