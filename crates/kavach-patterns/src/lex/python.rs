//! Python trivia cursor.
//!
//! `#` comments (outside strings only), `r/b/f/t/u` prefixes
//! (case-insensitive, pairs), triple quotes (multi-line), `\`
//! escapes + line continuation. Source: CPython lexical analysis docs.
use super::{RsKind, RsSeg, consume_escaped, consume_to_eol, consume_until, push};

/// Python source cursor. `f`/`t`-string `{…}` fields stay opaque `Str`
/// (re-lexing them is a follow-up).
#[must_use]
pub fn lex(src: &str) -> Vec<RsSeg> {
    let bytes = src.as_bytes();
    let mut out: Vec<RsSeg> = Vec::new();
    let mut pos = 0usize;
    let mut span_start = 0usize;
    while let Some(&byte) = bytes.get(pos) {
        if byte == b'#' {
            let end = consume_to_eol(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'\'' || byte == b'"' || is_py_prefix(bytes, pos) {
            let (start, triple, quote) = py_open(bytes, pos);
            let end = if triple {
                // Search past the opening delimiter itself.
                consume_until(bytes, start.saturating_add(3), quote, 3)
            } else {
                consume_escaped(bytes, start, quote)
            };
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        pos = pos.saturating_add(1);
    }
    push(&mut out, RsKind::Code, span_start, pos);
    out
}

/// True when `pos` opens a Python string: a quote, or 1–2 prefix letters
/// (`r/b/f/t/u`, any case) followed by a quote, with no ident char before.
fn is_py_prefix(bytes: &[u8], pos: usize) -> bool {
    if pos > 0
        && bytes
            .get(pos.saturating_sub(1))
            .copied()
            .is_some_and(|b| b.is_ascii_alphanumeric() && b != b'_')
    {
        return false;
    }
    let first = bytes.get(pos).copied().map(|b| b.to_ascii_lowercase());
    if !matches!(first, Some(b'r' | b'b' | b'f' | b't' | b'u')) {
        return false;
    }
    let second = bytes
        .get(pos.saturating_add(1))
        .copied()
        .map(|b| b.to_ascii_lowercase());
    match second {
        Some(b'\'' | b'"') => true,
        Some(s) => {
            matches!(s, b'r' | b'b' | b'f' | b't' | b'u')
                && matches!(
                    bytes.get(pos.saturating_add(2)).copied(),
                    Some(b'\'' | b'"')
                )
        }
        None => false,
    }
}

/// Python string open at `pos`: returns (content start, triple?, quote byte).
fn py_open(bytes: &[u8], pos: usize) -> (usize, bool, u8) {
    let mut cur = pos;
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\'' || byte == b'"' {
            break;
        }
        cur = cur.saturating_add(1);
    }
    let quote = bytes.get(cur).copied().unwrap_or(b'"');
    let triple = bytes.get(cur.saturating_add(1)).copied() == Some(quote)
        && bytes.get(cur.saturating_add(2)).copied() == Some(quote);
    (cur, triple, quote)
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::lex::{RsKind, text};

    fn joined(src: &str) -> String {
        lex(src).iter().map(|s| text(src, s)).collect::<Vec<_>>().join("")
    }

    #[test]
    fn hash_comment_splits_but_hash_in_string_does_not() {
        let src = "x = 1  # set x\ny = \"a#b\"  # ok\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let comments: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Comment)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(comments, vec!["# set x", "# ok"]);
    }

    #[test]
    fn triple_quoted_spans_lines() {
        let src = "s = '''line1\nline2 # not comment'''\nx = 1\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert_eq!(
            segs.iter().filter(|s| s.kind == RsKind::Comment).count(),
            0
        );
        assert_eq!(segs.iter().filter(|s| s.kind == RsKind::Str).count(), 1);
    }

    #[test]
    fn prefixes_lex_but_plain_idents_do_not() {
        let src = "a = rf\"x\"\nb = rb'y'\nraw = 1\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert_eq!(segs.iter().filter(|s| s.kind == RsKind::Str).count(), 2);
    }
}
