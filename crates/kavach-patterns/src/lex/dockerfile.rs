//! Dockerfile trivia cursor.
//!
//! `#` comments only at line start (leading whitespace allowed; parser
//! directives ride along as comments); `"…"` strings (single quotes are
//! literal); `\` line continuation is plain code. Source: Dockerfile
//! reference (BuildKit).
use super::{RsKind, RsSeg, consume_escaped, consume_to_eol, push};

/// Dockerfile source cursor.
#[must_use]
pub fn lex(src: &str) -> Vec<RsSeg> {
    let bytes = src.as_bytes();
    let mut out: Vec<RsSeg> = Vec::new();
    let mut pos = 0usize;
    let mut span_start = 0usize;
    while let Some(&byte) = bytes.get(pos) {
        if byte == b'#' && is_line_start(bytes, pos) {
            let end = consume_to_eol(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'"' {
            let end = consume_escaped(bytes, pos, b'"');
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

/// True when only whitespace precedes `pos` back to the previous newline.
fn is_line_start(bytes: &[u8], pos: usize) -> bool {
    let mut cur = pos;
    while cur > 0 {
        match bytes.get(cur.saturating_sub(1)).copied() {
            Some(b'\n') => return true,
            Some(b) if b == b' ' || b == b'\t' || b == b'\r' => {
                cur = cur.saturating_sub(1);
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::lex::{RsKind, text};

    fn joined(src: &str) -> String {
        lex(src).iter().map(|s| text(src, s)).collect::<Vec<_>>().join("")
    }

    #[test]
    fn hash_comment_needs_line_start() {
        // `a#b` mid-line is an argument, not a comment (Dockerfile reference).
        let src = "RUN echo a#b\n# real comment\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let comments: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Comment)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(comments, vec!["# real comment"]);
    }

    #[test]
    fn parser_directive_rides_as_comment() {
        let src = "# syntax=docker/dockerfile:1\nFROM alpine\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert_eq!(
            segs.iter().filter(|s| s.kind == RsKind::Comment).count(),
            1
        );
    }

    #[test]
    fn single_quotes_are_literal() {
        let src = "LABEL x='a#b'\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert!(!segs.iter().any(|s| s.kind == RsKind::Comment));
        assert!(!segs.iter().any(|s| s.kind == RsKind::Str));
    }
}
