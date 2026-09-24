//! Hub: multi-language trivia lexer for detectors (`lex/` leaves).
//!
//! One `Lang` enum + `segments_for` dispatch over the shared segment types
//! ([`RsKind`]/[`RsSeg`]), so detectors match on `Code` spans in any language
//! kavach scans. Per-language cursors live in the `lex/` leaves (modern
//! `foo`/`bar.rs` layout); shared byte-cursor helpers live here.
//!
//! Trivia tables grounded in primary sources (specs, collected 2026-09): Go
//! spec, JLS §3, Kotlin spec + strings docs, ECMA-262 + MDN, CPython lexical
//! analysis, POSIX shell, Dockerfile reference, PostgreSQL lexical docs.
//!
//! Documented approximations (v1): shell heredocs inside `$(…)` are not
//! tracked; Python f-string `{…}` fields nested inside other f-strings stay
//! opaque; regex-vs-division uses the standard operand-position heuristic
//! (`}` and line-start always allow regex — rare object-division cases
//! mislex, documented at `regex_opens`).
pub mod c_like;
pub mod dockerfile;
pub mod python;
pub(crate) mod rust;
pub mod shell;
pub mod sql;

/// Rust cursor, re-exported for detectors that scan Rust sources.
pub use rust::{code_words, segments};

use c_like::{C_GO, C_JAVA, C_KOTLIN, C_TSJS};

/// Languages kavach scans (mirrors `file_types` classifiers + SQL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Lang {
    /// Rust (frozen [`rust`] cursor, pinned by its tests).
    Rust,
    /// Go: `//`, non-nesting `/* */`, `"…"` + backquote raw + `'x'` runes.
    Go,
    /// Java: `//`, non-nesting `/* */`, `"…"`, `'c'`, `"""` text blocks.
    Java,
    /// Kotlin: `//`, NESTING `/* */`, `"…"` + `"""` raw, `$` templates.
    Kotlin,
    /// TypeScript/JavaScript (tsx/jsx/astro): `//`, non-nesting `/* */`,
    /// `#!` hashbang, `"…"`, `'…'`, `` `…${…}…` `` templates.
    TsJs,
    /// Python: `#` comments, `r/b/f/t/u` prefixes, triple quotes.
    Python,
    /// sh/bash/zsh: word-start `#`, `'...'` literal, `"…"` + `$'…'`, `$(…)`.
    Shell,
    /// Dockerfile: line-start `#` (+ directives), `"…"`, `\` continuation.
    Dockerfile,
    /// SQL: `--` line, NESTING `/* */`, `'…'` (`''` escape), `E'…'`,
    /// `$tag$…$tag$`, `"…"` quoted identifiers (folded into `Str`).
    Sql,
}

/// Segment class: executable code, literal text, or comment prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RsKind {
    /// Executable code (keywords, idents, symbols, lifetimes).
    Code,
    /// String, byte-string, raw-string, template, or char literal.
    Str,
    /// Line comment or block comment (nesting per language).
    Comment,
}

/// One classified byte span. `Copy` (12 bytes); text recovered via [`text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RsSeg {
    /// What the span is.
    pub kind: RsKind,
    /// Start byte offset into the source.
    pub start: u32,
    /// End byte offset into the source.
    pub end: u32,
}

/// Zero-copy text of a segment.
#[must_use]
pub fn text<'a>(src: &'a str, seg: &RsSeg) -> &'a str {
    src.get(seg.start as usize..seg.end as usize).unwrap_or("")
}

/// 1-based line number for a byte offset (detector reports).
#[must_use]
pub fn line_no(src: &str, offset: u32) -> usize {
    src.get(..offset as usize).map_or(1, |head| {
        head.bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            .saturating_add(1)
    })
}

/// Code-only text of a Rust source for whole-content regex oracles.
///
/// `Code` spans verbatim, `Str` spans dropped, comments replaced by a space
/// (or a newline when they span lines — the Go-spec rule, so `a/*\n*/b`
/// keeps its line break and `a/*x*/b` keeps its word gap instead of fusing
/// to `ab`). Other languages: `segments_for` + [`code_text_for`].
#[must_use]
pub fn code_text(src: &str) -> String {
    code_text_for(src, &segments(src))
}

/// Code-only text over precomputed segments (per-line callers lex once and
/// slice per line instead).
#[must_use]
pub fn code_text_for(src: &str, segs: &[RsSeg]) -> String {
    let mut out = String::with_capacity(src.len());
    for seg in segs {
        match seg.kind {
            RsKind::Code => out.push_str(text(src, seg)),
            RsKind::Str => {}
            RsKind::Comment => {
                if text(src, seg).contains('\n') {
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
        }
    }
    out
}

/// Code-only view of ONE line for per-line regex oracles.
///
/// `Code` spans verbatim, every other span replaced by a single space (no
/// newlines can occur, so line numbering never drifts). Payload rules that
/// need string contents stay on the raw line — this is strictly the
/// code-match view.
#[must_use]
pub fn code_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for seg in segments(line) {
        match seg.kind {
            RsKind::Code => out.push_str(text(line, &seg)),
            RsKind::Str | RsKind::Comment => out.push(' '),
        }
    }
    out
}

/// Extension dispatch for trivia lexing.
///
/// Reuses the `file_types` classifiers; `.kt`/`.kts` split from Java,
/// unknown extensions fall back to the C-like TS/JS core (most common
/// denominator), documented at the call site.
#[must_use]
pub fn lang_for_path(path: &str) -> Lang {
    if super::file_types::is_rust_file(path) {
        return Lang::Rust;
    }
    if super::file_types::is_go_file(path) {
        return Lang::Go;
    }
    if super::file_types::is_python_file(path) {
        return Lang::Python;
    }
    if super::file_types::is_shell_file(path) {
        return Lang::Shell;
    }
    if super::file_types::is_dockerfile(path) {
        return Lang::Dockerfile;
    }
    if super::file_types::is_frontend_file(path) {
        return Lang::TsJs;
    }
    if is_kotlin_path(path) {
        return Lang::Kotlin;
    }
    if is_java_path(path) {
        return Lang::Java;
    }
    if is_sql_path(path) {
        return Lang::Sql;
    }
    Lang::TsJs
}

/// Classify a whole source with the trivia table for `lang`.
#[must_use]
pub fn segments_for(lang: Lang, src: &str) -> Vec<RsSeg> {
    match lang {
        Lang::Rust => segments(src),
        Lang::Go => c_like::lex(src, C_GO),
        Lang::Java => c_like::lex(src, C_JAVA),
        Lang::Kotlin => c_like::lex(src, C_KOTLIN),
        Lang::TsJs => c_like::lex(src, C_TSJS),
        Lang::Python => python::lex(src),
        Lang::Shell => shell::lex(src),
        Lang::Dockerfile => dockerfile::lex(src),
        Lang::Sql => sql::lex(src),
    }
}

/// `.kt`/`.kts` split from Java (which shares the `is_java_file` classifier).
fn is_kotlin_path(path: &str) -> bool {
    use std::path::Path;
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("kt") || ext.eq_ignore_ascii_case("kts"))
}

/// `.java` (Kotlin already excluded above).
fn is_java_path(path: &str) -> bool {
    use std::path::Path;
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("java"))
}

/// `.sql` sources.
fn is_sql_path(path: &str) -> bool {
    use std::path::Path;
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
}

// ——— shared byte-cursor helpers (one impl, all leaves) ———

/// Push one span unless empty.
pub(crate) fn push(out: &mut Vec<RsSeg>, kind: RsKind, start: usize, end: usize) {
    if end > start {
        out.push(RsSeg {
            kind,
            start: sat_u32(start),
            end: sat_u32(end),
        });
    }
}

/// Saturating `usize` → `u32` for span storage.
pub(crate) fn sat_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Consume to (not past) the end of line.
pub(crate) fn consume_to_eol(bytes: &[u8], pos: usize) -> usize {
    let mut end = pos;
    while bytes.get(end).copied().is_some_and(|b| b != b'\n') {
        end = end.saturating_add(1);
    }
    end
}

/// Consume a `/* … */` comment; `nest` selects Kotlin/SQL nesting vs
/// Go/Java/TS single-level (first `*/` wins).
pub(crate) fn consume_nested(bytes: &[u8], pos: usize, nest: bool) -> usize {
    let mut depth = 1usize;
    let mut end = pos.saturating_add(2);
    while end < bytes.len() && depth > 0 {
        let pair = (
            bytes.get(end).copied(),
            bytes.get(end.saturating_add(1)).copied(),
        );
        match pair {
            (Some(b'/'), Some(b'*')) if nest => {
                depth = depth.saturating_add(1);
                end = end.saturating_add(2);
            }
            (Some(b'*'), Some(b'/')) => {
                depth = depth.saturating_sub(1);
                end = end.saturating_add(2);
            }
            (Some(_), _) => end = end.saturating_add(1),
            (None, _) => break,
        }
    }
    end
}

/// Consume a quoted run from the opening quote at `pos` with `\` escapes;
/// single-line forms end at the newline when unterminated.
pub(crate) fn consume_escaped(bytes: &[u8], pos: usize, quote: u8) -> usize {
    let mut cur = pos.saturating_add(1);
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\\' {
            cur = cur.saturating_add(2);
        } else if byte == quote {
            return cur.saturating_add(1);
        } else if byte == b'\n' {
            return cur;
        } else {
            cur = cur.saturating_add(1);
        }
    }
    cur
}

/// Consume a run terminated by `count` consecutive `quote` bytes
/// (raw strings, triple quotes, backquotes); newlines allowed.
pub(crate) fn consume_until(bytes: &[u8], mut cur: usize, quote: u8, count: usize) -> usize {
    while cur < bytes.len() {
        if run_of(bytes, cur, quote, count) {
            return cur.saturating_add(count);
        }
        cur = cur.saturating_add(1);
    }
    cur
}

/// True when `count` copies of `quote` start at `cur`.
pub(crate) fn run_of(bytes: &[u8], cur: usize, quote: u8, count: usize) -> bool {
    for i in 0..count {
        if bytes.get(cur.saturating_add(i)).copied() != Some(quote) {
            return false;
        }
    }
    true
}

/// Skip a quoted run inside a brace/paren island (no nesting inside).
pub(crate) fn skip_quoted_island(bytes: &[u8], pos: usize, quote: u8) -> usize {
    let mut cur = pos.saturating_add(1);
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\\' {
            cur = cur.saturating_add(2);
        } else if byte == quote {
            return cur.saturating_add(1);
        } else {
            cur = cur.saturating_add(1);
        }
    }
    cur
}
