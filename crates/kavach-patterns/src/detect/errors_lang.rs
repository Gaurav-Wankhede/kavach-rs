//! Language-specific error handling anti-patterns.
use std::sync::OnceLock;

use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::{is_go_file, is_rust_file};
use crate::regex_patterns::fbase;
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

/// Detect Rust error handling issues.
pub(super) fn detect_rust_errors(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if !is_rust_file(fp) {
        return;
    }
    if idx_or_skip(r, 16).is_match(content) && crate::file_types::is_handler_file(fp) {
        add(
            res,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "unwrap-in-handler",
            "Use ?.",
        );
    }
    ck(
        res,
        idx_or_skip(r, 26),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "expect-generic",
        "Rich message.",
    );
    if idx_or_skip(r, 27).find_iter(content).count() > 10 {
        add(
            res,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "excessive-clone",
            "Borrow instead.",
        );
    }
}

/// Detect Go error handling issues.
pub(super) fn detect_go_errors(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if !is_go_file(fp) {
        return;
    }
    if idx_or_skip(r, 29).is_match(content) && fbase(fp) != "main.go" {
        add(
            res,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "go-abort",
            "Return error.",
        );
    }
    ck(
        res,
        idx_or_skip(r, 30),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "_ = (discard)",
        "Handle error.",
    );
}

/// Detect Python error handling issues.
pub(super) fn detect_python_errors(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,
        idx_or_skip(r, 34),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "bare except",
        "Handle explicitly.",
    );
    ck(
        res,
        idx_or_skip(r, 35),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "type: ignore",
        "Add annotation.",
    );
}

/// Detect Java error handling issues.
pub(super) fn detect_java_errors(res: &mut Vec<AntiProdResult>, r: &[Regex], content: &str) {
    ck(
        res,
        idx_or_skip(r, 39),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "empty catch",
        "Handle exception.",
    );
}

/// Detect Docker error handling issues.
pub(super) fn detect_docker_errors(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if crate::file_types::is_dockerfile(fp) {
        ck(
            res,
            idx_or_skip(r, 44),
            content,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "ADD not COPY",
            "Use COPY.",
        );
    }
}
