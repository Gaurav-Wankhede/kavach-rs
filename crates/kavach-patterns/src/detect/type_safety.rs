//! Type safety and linting suppression anti-patterns.
use std::sync::OnceLock;

use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::{is_frontend_file, is_go_file, is_java_file, is_python_file, is_rust_file};
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

/// Detect type safety issues and suppression directives.
pub(super) fn detect_type_safety(
    res: &mut Vec<AntiProdResult>,
    r: &[Regex],
    fp: &str,
    content: &str,
) {
    if is_frontend_file(fp) {
        ck(
            res,
            idx_or_skip(r, 7),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "as any",
            "Type narrow.",
        );
        ck(
            res,
            idx_or_skip(r, 8),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "@ts-ignore",
            "Fix type.",
        );
        ck(
            res,
            idx_or_skip(r, 9),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "eslint-disable",
            "Fix lint.",
        );
        ck(
            res,
            idx_or_skip(r, 10),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "@ts-nocheck",
            "Fix types.",
        );
    }
    if is_go_file(fp) {
        ck(
            res,
            idx_or_skip(r, 31),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "nolint",
            "Fix lint.",
        );
    }
    if is_python_file(fp) {
        ck(
            res,
            idx_or_skip(r, 36),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "noqa",
            "Fix lint.",
        );
    }
    if is_java_file(fp) {
        ck(
            res,
            idx_or_skip(r, 40),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "@SuppressWarnings",
            "Fix types.",
        );
    }
    if is_rust_file(fp) {
        ck(
            res,
            idx_or_skip(r, 23),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "allow(dead_code)",
            "Remove dead.",
        );
        ck(
            res,
            idx_or_skip(r, 24),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "allow(unused)",
            "Use or remove.",
        );
        ck(
            res,
            idx_or_skip(r, 25),
            content,
            AntiProdLevel::P3TypeLoose,
            "TYPE_LOOSE",
            "allow(clippy::)",
            "Fix warning.",
        );
    }
}
