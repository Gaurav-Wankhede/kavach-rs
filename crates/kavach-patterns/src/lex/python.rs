//! Python trivia cursor.
//!
//! `#` comments (outside strings only), `r/b/f/t/u` prefixes
//! (case-insensitive, pairs), triple quotes (multi-line), `\`
//! escapes + line continuation, `f`/`t`-string `{…}` fields re-lexed as code.
//! Source: CPython lexical analysis docs.
use super::{RsKind, RsSeg, consume_escaped, consume_to_eol, consume_until, push};

/// Python source cursor.
///
/// `f`/`t`-string replacement fields are re-lexed: field code as `Code`,
/// `#` comments inside fields as `Comment`, nested strings as `Str` (one
/// level — fields inside nested f-strings stay opaque).
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
            if is_fstring(bytes, pos, start) {
                lex_fstring(bytes, &mut out, pos, start, end, triple, quote);
            } else {
                push(&mut out, RsKind::Str, pos, end);
            }
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

/// True when the literal opening at `pos` (prefix start) through `quote_pos`
/// carries an `f`/`F`/`t`/`T` prefix (formatted/template string).
fn is_fstring(bytes: &[u8], pos: usize, quote_pos: usize) -> bool {
    let mut cur = pos;
    while cur < quote_pos {
        match bytes.get(cur).copied() {
            Some(b'f' | b'F' | b't' | b'T') => return true,
            _ => cur = cur.saturating_add(1),
        }
    }
    false
}

/// Emit an f/t-string span: literal chunks as `Str`, `{…}` fields re-lexed
/// (field code as `Code`, `#` comments as `Comment`, nested strings as
/// `Str`). `{{`/`}}` are literal-brace escapes staying `Str`.
fn lex_fstring(
    bytes: &[u8],
    out: &mut Vec<RsSeg>,
    lit_start: usize,
    content_start: usize,
    end: usize,
    triple: bool,
    quote: u8,
) {
    // Content bounds sans delimiters (unterminated spans simply run to EOF).
    let open_len = if triple { 3 } else { 1 };
    let mut close_len = 0usize;
    if triple && end >= 3 && bytes.get(end.saturating_sub(3)).copied() == Some(quote) {
        close_len = 3;
    } else if !triple && end > 0 && bytes.get(end.saturating_sub(1)).copied() == Some(quote) {
        close_len = 1;
    }
    let content_end = end.saturating_sub(close_len);
    let mut chunk = lit_start;
    let mut cur = content_start.saturating_add(open_len);
    while cur < content_end {
        match bytes.get(cur).copied() {
            Some(b'{') if bytes.get(cur.saturating_add(1)).copied() == Some(b'{') => {
                cur = cur.saturating_add(2);
            }
            Some(b'}') if bytes.get(cur.saturating_add(1)).copied() == Some(b'}') => {
                cur = cur.saturating_add(2);
            }
            Some(b'{') => {
                push(out, RsKind::Str, chunk, cur);
                cur = lex_field(bytes, out, cur, content_end);
                chunk = cur;
            }
            _ => {
                cur = cur.saturating_add(1);
            }
        }
    }
    push(out, RsKind::Str, chunk, end);
}

/// Re-lex one `{…}` field starting at the open brace: balanced braces with
/// quote-skipping; returns the offset past the closing `}` (or content end).
/// Interior: strings → `Str`, `#`-to-EOL → `Comment`, rest `Code` (braces ride
/// with the surrounding `Code`).
fn lex_field(bytes: &[u8], out: &mut Vec<RsSeg>, open: usize, content_end: usize) -> usize {
    // Find the matching close first (nesting + quote aware).
    let mut depth = 0usize;
    let mut cur = open;
    let mut close = content_end;
    while cur < content_end {
        match bytes.get(cur).copied() {
            Some(b'"' | b'\'') => {
                cur = skip_py_string(bytes, cur);
            }
            Some(b'{') => {
                depth = depth.saturating_add(1);
                cur = cur.saturating_add(1);
            }
            Some(b'}') => {
                if bytes.get(cur.saturating_add(1)).copied() == Some(b'}') {
                    cur = cur.saturating_add(2);
                } else if depth <= 1 {
                    close = cur;
                    break;
                } else {
                    depth = depth.saturating_sub(1);
                    cur = cur.saturating_add(1);
                }
            }
            _ => {
                cur = cur.saturating_add(1);
            }
        }
    }
    // Both braces ride as Code so spans tile losslessly (in offset order).
    push(out, RsKind::Code, open, open.saturating_add(1));
    partition_field(bytes, out, open.saturating_add(1), close);
    if close < content_end {
        push(out, RsKind::Code, close, close.saturating_add(1));
        close.saturating_add(1)
    } else {
        close
    }
}

/// Partition field interior `[start, end)`: strings → `Str`, `#`-to-EOL →
/// `Comment`, everything else `Code`.
fn partition_field(bytes: &[u8], out: &mut Vec<RsSeg>, start: usize, end: usize) {
    let mut chunk = start;
    let mut cur = start;
    while cur < end {
        match bytes.get(cur).copied() {
            Some(b'#') => {
                push(out, RsKind::Code, chunk, cur);
                let hash = cur;
                while cur < end && bytes.get(cur).copied() != Some(b'\n') {
                    cur = cur.saturating_add(1);
                }
                push(out, RsKind::Comment, hash, cur);
                chunk = cur;
            }
            Some(b'"' | b'\'') => {
                push(out, RsKind::Code, chunk, cur);
                let str_start = cur;
                cur = skip_py_string(bytes, cur);
                push(out, RsKind::Str, str_start, cur);
                chunk = cur;
            }
            _ => {
                cur = cur.saturating_add(1);
            }
        }
    }
    push(out, RsKind::Code, chunk, end);
}

/// Skip a (possibly prefixed/triple) string starting at a quote byte;
/// returns the offset past its end. Approximates prefixes by scanning back
/// at most 2 prefix letters.
fn skip_py_string(bytes: &[u8], quote_pos: usize) -> usize {
    let Some(quote) = bytes.get(quote_pos).copied() else {
        return quote_pos.saturating_add(1);
    };
    let triple = bytes.get(quote_pos.saturating_add(1)).copied() == Some(quote)
        && bytes.get(quote_pos.saturating_add(2)).copied() == Some(quote);
    if triple {
        consume_until(bytes, quote_pos.saturating_add(3), quote, 3)
    } else {
        consume_escaped(bytes, quote_pos, quote)
    }
}
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

    #[test]
    fn fstring_field_code_is_code() {
        let src = "s = f\"hi {user.name}!\";\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("{user.name}"), "got: {code}");
    }

    #[test]
    fn fstring_comment_in_field_is_comment() {
        let src = "s = f\"{x # why\n}\";\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert!(segs.iter().any(|s| s.kind == RsKind::Comment));
    }

    #[test]
    fn fstring_literal_braces_stay_str() {
        let src = "s = f\"{{not a field}}\";\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert!(!segs.iter().any(|s| s.kind == RsKind::Comment));
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(!code.contains("not a field"), "got: {code}");
    }

    #[test]
    fn plain_strings_have_no_fields() {
        let src = "s = \"{not code}\";\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(!code.contains("not code"), "got: {code}");
    }
}
