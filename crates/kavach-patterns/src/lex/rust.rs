//! Rust trivia cursor (frozen): `//` + nesting `/* */`, `"…"`, `r#"…"#`,
//! `b"…"` literals, char-vs-lifetime heuristic. Pinned by `rust_test.rs`;
//! other languages live in the sibling leaves.
//!
//! Byte-driven (all Rust syntax characters are ASCII), so offsets are always
//! char boundaries and multi-byte text passes through untouched.
use super::{RsKind, RsSeg, push, text};

/// Classify a whole Rust source into spans. Single pass, no allocation per
/// token — one `Vec` of 12-byte spans for the file.
#[must_use]
pub fn segments(src: &str) -> Vec<RsSeg> {
    let bytes = src.as_bytes();
    let mut out: Vec<RsSeg> = Vec::new();
    let mut pos = 0usize;
    let mut span_start = 0usize;

    while let Some(&byte) = bytes.get(pos) {
        let ahead = bytes.get(pos.saturating_add(1)).copied();
        if byte == b'/' && (ahead == Some(b'/') || ahead == Some(b'*')) {
            let end = if ahead == Some(b'/') {
                consume_line_comment(bytes, pos)
            } else {
                consume_block_comment(bytes, pos)
            };
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'"' {
            let end = consume_quoted(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if (byte == b'r' || byte == b'b') && is_literal_prefix(bytes, pos) {
            let end = consume_prefixed(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'\'' {
            if let Some(end) = char_literal_end(bytes, pos) {
                push(&mut out, RsKind::Code, span_start, pos);
                push(&mut out, RsKind::Str, pos, end);
                pos = end;
                span_start = pos;
                continue;
            }
            // Lifetime or separator: plain code byte.
            pos = pos.saturating_add(1);
            continue;
        }
        pos = pos.saturating_add(1);
    }
    push(&mut out, RsKind::Code, span_start, pos);
    out
}

/// Identifier-ish words inside `Code` spans, in order, borrowed. Powers
/// keyword detection (`fn`, `struct`, `use`, …) without substring scans.
#[must_use]
pub fn code_words<'a>(src: &'a str, segs: &[RsSeg]) -> Vec<&'a str> {
    let mut out = Vec::new();
    for seg in segs.iter().filter(|s| s.kind == RsKind::Code) {
        let body = text(src, seg);
        let mut flush_word = |start: usize, end: usize| {
            if let Some(word) = body.get(start..end) {
                out.push(word);
            }
        };
        let mut start: Option<usize> = None;
        for (i, byte) in body.bytes().enumerate() {
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                if start.is_none() {
                    start = Some(i);
                }
            } else if let Some(word_start) = start.take() {
                flush_word(word_start, i);
            }
        }
        if let Some(word_start) = start {
            flush_word(word_start, body.len());
        }
    }
    out
}

/// Byte at `pos`, or `None` past EOF.
fn at(bytes: &[u8], pos: usize) -> Option<u8> {
    bytes.get(pos).copied()
}

/// Consume a `//` line comment; returns the offset past the newline (or EOF).
fn consume_line_comment(bytes: &[u8], pos: usize) -> usize {
    let mut end = pos.saturating_add(2);
    while at(bytes, end).is_some_and(|b| b != b'\n') {
        end = end.saturating_add(1);
    }
    end
}

/// Consume a `/* … */` block comment with nesting; returns the offset past the
/// final `/` (or EOF on unterminated input).
fn consume_block_comment(bytes: &[u8], pos: usize) -> usize {
    let mut depth = 1usize;
    let mut end = pos.saturating_add(2);
    while end < bytes.len() && depth > 0 {
        match (at(bytes, end), at(bytes, end.saturating_add(1))) {
            (Some(b'/'), Some(b'*')) => {
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

/// Consume a `"`-quoted string with `\` escapes; returns the offset past the
/// closing quote (or EOF on unterminated input).
fn consume_quoted(bytes: &[u8], pos: usize) -> usize {
    consume_quoted_tail(bytes, pos.saturating_add(1))
}

/// Tail of a `"`-string once the opening quote is consumed.
fn consume_quoted_tail(bytes: &[u8], mut cur: usize) -> usize {
    while let Some(byte) = at(bytes, cur) {
        if byte == b'\\' {
            cur = cur.saturating_add(2);
        } else if byte == b'"' {
            return cur.saturating_add(1);
        } else {
            cur = cur.saturating_add(1);
        }
    }
    cur
}

/// True when `bytes[pos]` (`r` or `b`) opens a prefixed literal rather than an
/// identifier: the previous byte is not an ident char, and a quote form
/// (`"`, `#*"`, `r#*"`) follows.
fn is_literal_prefix(bytes: &[u8], pos: usize) -> bool {
    if pos > 0
        && at(bytes, pos.saturating_sub(1)).is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return false;
    }
    // Skip the optional `b`, then the optional `r`, then optional `#`s.
    let mut cur = pos;
    if at(bytes, cur) == Some(b'b') {
        cur = cur.saturating_add(1);
    }
    if at(bytes, cur) == Some(b'r') {
        // A lone `r` must be followed by `"` or `#` (else it is an ident).
        cur = cur.saturating_add(1);
        while at(bytes, cur) == Some(b'#') {
            cur = cur.saturating_add(1);
        }
        return at(bytes, cur) == Some(b'"');
    }
    at(bytes, cur) == Some(b'"')
}

/// Consume a prefixed literal (`r"…"`, `r#"…"#`, `b"…"`, `br"…"`) starting at
/// `pos`; returns the offset past its end (or EOF). Always advances.
fn consume_prefixed(bytes: &[u8], pos: usize) -> usize {
    let mut cur = pos.saturating_add(1);
    if at(bytes, cur) == Some(b'r') && at(bytes, pos) == Some(b'b') {
        cur = cur.saturating_add(1);
    }
    let mut hashes = 0usize;
    while at(bytes, cur) == Some(b'#') {
        hashes = hashes.saturating_add(1);
        cur = cur.saturating_add(1);
    }
    if at(bytes, cur) != Some(b'"') {
        return cur;
    }
    cur = cur.saturating_add(1);
    if hashes == 0 {
        return consume_quoted_tail(bytes, cur);
    }
    while cur < bytes.len() {
        if at(bytes, cur) == Some(b'"') {
            let mut ok = true;
            for i in 1..=hashes {
                if at(bytes, cur.saturating_add(i)) != Some(b'#') {
                    ok = false;
                    break;
                }
            }
            if ok {
                return cur.saturating_add(hashes).saturating_add(1);
            }
        }
        cur = cur.saturating_add(1);
    }
    cur
}

/// End offset (exclusive) when `bytes[pos]` opens a char literal, else `None`.
///
/// A `'` opens a char literal only if a closing `'` follows on the same line
/// with no whitespace inside (`'a'`, `'\n'`, `'\u{1F600}'`); otherwise it is a
/// lifetime or separator and stays code. A space-containing literal (`' '`)
/// conservatively stays code — spaces are detector-inert.
fn char_literal_end(bytes: &[u8], pos: usize) -> Option<usize> {
    let mut cur = pos.saturating_add(1);
    let mut len = 0usize;
    while cur < bytes.len() && len < 12 {
        match at(bytes, cur) {
            None | Some(b'\n') => return None,
            Some(b'\'') => {
                return if len == 0 {
                    None
                } else {
                    Some(cur.saturating_add(1))
                };
            }
            Some(byte) if byte.is_ascii_whitespace() => return None,
            _ => {
                cur = cur.saturating_add(1);
                len = len.saturating_add(1);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "rust_test.rs"]
mod rust_test;
