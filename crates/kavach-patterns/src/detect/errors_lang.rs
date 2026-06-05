//! Language-specific error handling anti-patterns.
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::{is_go_file, is_rust_file};
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
    if idx(r, 16).is_match(content) && crate::file_types::is_handler_file(fp) {
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
        idx(r, 26),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "expect-generic",
        "Rich message.",
    );
    if idx(r, 27).find_iter(content).count() > 10 {
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
    if idx(r, 29).is_match(content) && fbase(fp) != "main.go" {
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
        idx(r, 30),
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
        idx(r, 34),
        content,
        AntiProdLevel::P2ErrorBlind,
        "ERROR_BLIND",
        "bare except",
        "Handle explicitly.",
    );
    ck(
        res,
        idx(r, 35),
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
        idx(r, 39),
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
            idx(r, 44),
            content,
            AntiProdLevel::P2ErrorBlind,
            "ERROR_BLIND",
            "ADD not COPY",
            "Use COPY.",
        );
    }
}
