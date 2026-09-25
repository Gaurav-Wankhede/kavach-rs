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
/// `#` comments inside fields as `Comment`, nested strings as `Str`, nested
/// f-strings recurse (depth-capped).
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
                lex_fstring(
                    bytes,
                    &mut out,
                    FStrSpan {
                        lit_start: pos,
                        content_start: start,
                        end,
                        triple,
                        quote,
                    },
                    0,
                );
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

/// Max f-string nesting re-lexed (CPython allows arbitrary nesting; past
/// this depth fields stay opaque `Str` — spans still tile losslessly).
const FSTRING_MAX_DEPTH: usize = 3;

/// One f/t-string literal's bounds: outer span plus content span sans
/// delimiters, triple flag, and quote byte.
#[derive(Debug, Clone, Copy)]
struct FStrSpan {
    /// Literal start (prefix included).
    lit_start: usize,
    /// Content start (past prefix + opening quotes).
    content_start: usize,
    /// Literal end (past closing quotes, or EOF when unterminated).
    end: usize,
    /// Triple-quoted form.
    triple: bool,
    /// Quote byte.
    quote: u8,
}

/// Emit an f/t-string span: literal chunks as `Str`, `{…}` fields re-lexed
/// (field code as `Code`, `#` comments as `Comment`, nested strings as
/// `Str`). `{{`/`}}` are literal-brace escapes staying `Str`. Past
/// [`FSTRING_MAX_DEPTH`] the whole literal stays opaque `Str`.
fn lex_fstring(bytes: &[u8], out: &mut Vec<RsSeg>, span: FStrSpan, depth: usize) {
    if depth > FSTRING_MAX_DEPTH {
        push(out, RsKind::Str, span.lit_start, span.end);
        return;
    }
    // Content bounds sans delimiters (unterminated spans simply run to EOF).
    let open_len = if span.triple { 3 } else { 1 };
    let mut close_len = 0usize;
    if span.triple
        && span.end >= 3
        && bytes.get(span.end.saturating_sub(3)).copied() == Some(span.quote)
    {
        close_len = 3;
    } else if !span.triple
        && span.end > 0
        && bytes.get(span.end.saturating_sub(1)).copied() == Some(span.quote)
    {
        close_len = 1;
    }
    let content_end = span.end.saturating_sub(close_len);
    let mut chunk = span.lit_start;
    let mut cur = span.content_start.saturating_add(open_len);
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
                cur = lex_field(bytes, out, cur, content_end, depth);
                chunk = cur;
            }
            _ => {
                cur = cur.saturating_add(1);
            }
        }
    }
    push(out, RsKind::Str, chunk, span.end);
}

/// Re-lex one `{…}` field starting at the open brace: balanced braces with
/// quote-skipping; returns the offset past the closing `}` (or content end).
/// Interior: strings → `Str` (nested f-strings recurse while `depth` allows),
/// `#`-to-EOL → `Comment`, rest `Code` (braces ride with surrounding `Code`).
fn lex_field(
    bytes: &[u8],
    out: &mut Vec<RsSeg>,
    open: usize,
    content_end: usize,
    depth: usize,
) -> usize {
    // Find the matching close first (nesting + quote aware).
    let mut brace_depth = 0usize;
    let mut cur = open;
    let mut close = content_end;
    while cur < content_end {
        match bytes.get(cur).copied() {
            Some(b'"' | b'\'') => {
                cur = skip_py_string(bytes, cur);
            }
            Some(b'{') => {
                brace_depth = brace_depth.saturating_add(1);
                cur = cur.saturating_add(1);
            }
            Some(b'}') => {
                if bytes.get(cur.saturating_add(1)).copied() == Some(b'}') {
                    cur = cur.saturating_add(2);
                } else if brace_depth <= 1 {
                    close = cur;
                    break;
                } else {
                    brace_depth = brace_depth.saturating_sub(1);
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
    partition_field(bytes, out, open.saturating_add(1), close, depth);
    if close < content_end {
        push(out, RsKind::Code, close, close.saturating_add(1));
        close.saturating_add(1)
    } else {
        close
    }
}

/// Partition field interior `[start, end)`: nested f-strings recurse (depth
/// capped), other strings → `Str`, `#`-to-EOL → `Comment`, rest `Code`.
fn partition_field(bytes: &[u8], out: &mut Vec<RsSeg>, start: usize, end: usize, depth: usize) {
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
                if let Some((lit, content, close, triple, quote)) = nested_fstring(bytes, cur, end)
                {
                    // Nested f-string: pending code ends before its prefix
                    // letters so no byte emits twice, then recurse.
                    push(out, RsKind::Code, chunk, lit);
                    lex_fstring(
                        bytes,
                        out,
                        FStrSpan {
                            lit_start: lit,
                            content_start: content,
                            end: close,
                            triple,
                            quote,
                        },
                        depth.saturating_add(1),
                    );
                    cur = close;
                    chunk = cur;
                    continue;
                }
                // Plain string: flush pending code first, then the literal,
                // clamped to the field bound (an unterminated quote must not
                // swallow the enclosing span's end and get rescanned).
                push(out, RsKind::Code, chunk, cur);
                let str_start = cur;
                cur = skip_py_string(bytes, cur).min(end);
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

/// Bounds of a nested f/t-string whose opening quote sits at `quote_pos`
/// (prefix letters immediately before it): `(lit_start, content_start, end,
/// triple, quote)`. `None` for plain strings or when the prefix check fails.
/// The end is found with the same triple/escape rules (bounded by `bound`).
fn nested_fstring(
    bytes: &[u8],
    quote_pos: usize,
    bound: usize,
) -> Option<(usize, usize, usize, bool, u8)> {
    let quote = bytes.get(quote_pos).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    // Prefix: up to 2 f/t letters directly before the quote, ident-bounded.
    let mut lit = quote_pos;
    let mut seen_format = false;
    for _ in 0..2 {
        if lit == 0 {
            break;
        }
        match bytes.get(lit.saturating_sub(1)).copied() {
            Some(b'f' | b'F' | b't' | b'T') => {
                seen_format = true;
                lit = lit.saturating_sub(1);
            }
            Some(b'r' | b'R' | b'b' | b'B' | b'u' | b'U') => {
                lit = lit.saturating_sub(1);
            }
            _ => break,
        }
    }
    if !seen_format {
        return None;
    }
    if lit > 0
        && bytes
            .get(lit.saturating_sub(1))
            .copied()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    let triple = bytes.get(quote_pos.saturating_add(1)).copied() == Some(quote)
        && bytes.get(quote_pos.saturating_add(2)).copied() == Some(quote);
    let content_start = quote_pos;
    let mut end = if triple {
        consume_until(bytes, quote_pos.saturating_add(3), quote, 3)
    } else {
        consume_escaped(bytes, quote_pos, quote)
    };
    if end > bound {
        end = bound;
    }
    Some((lit, content_start, end, triple, quote))
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
    let Some(quote) = bytes.get(cur).copied() else {
        // Unreachable through `is_py_prefix` (a quote always follows), but
        // total: fall back to a plain double-quote open at `cur`.
        return (cur, false, b'"');
    };
    let triple = bytes.get(cur.saturating_add(1)).copied() == Some(quote)
        && bytes.get(cur.saturating_add(2)).copied() == Some(quote);
    (cur, triple, quote)
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::lex::{RsKind, text};
    fn joined(src: &str) -> String {
        lex(src)
            .iter()
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("")
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
        assert_eq!(segs.iter().filter(|s| s.kind == RsKind::Comment).count(), 0);
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

    #[test]
    fn unterminated_quote_in_field_clamps_to_bound() {
        // Fuzzer-found: a string skip escaping its field bound, then the
        // outer cursor rescanning those bytes (overlap).
        let src = "T\"A{'\"";
        let segs = lex(src);
        let mut prev = 0usize;
        for seg in &segs {
            let (start, end) = (seg.start as usize, seg.end as usize);
            assert!(start >= prev, "overlap in {src:?}: {segs:?}");
            prev = end;
        }
        assert_eq!(prev, src.len(), "gap in {src:?}");
    }

    #[test]
    fn nested_fstring_fields_recurse() {
        let src = "s = f\"outer {f'{x}'} done\";\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("{x}"), "got: {code}");
    }
}
