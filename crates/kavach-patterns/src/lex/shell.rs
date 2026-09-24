//! Shell trivia cursor (sh/bash/zsh).
//!
//! `#` comments only at word start (POSIX: `a#b` is literal); `'...'`
//! fully literal; `"…"` + `$'…'`; `$(…)` nests as code islands; backquote
//! runs are command text (`Code`); `\` escapes + continuation. Heredocs are
//! NOT tracked (bodies read as code, as before).
use super::{RsKind, RsSeg, consume_escaped, consume_to_eol, consume_until, push, skip_quoted_island};

/// Shell source cursor.
#[must_use]
pub fn lex(src: &str) -> Vec<RsSeg> {
    let bytes = src.as_bytes();
    let mut out: Vec<RsSeg> = Vec::new();
    let mut pos = 0usize;
    let mut span_start = 0usize;
    while let Some(&byte) = bytes.get(pos) {
        if byte == b'#' && is_word_start(bytes, pos) {
            let end = consume_to_eol(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'\'' {
            let end = consume_until(bytes, pos.saturating_add(1), b'\'', 1);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'"' {
            let end = consume_escaped(bytes, pos, b'"');
            push(&mut out, RsKind::Code, span_start, pos);
            // `$(…)` and backquotes stay live inside `"…"` (POSIX): split
            // command islands out as Code, remainder stays Str.
            emit_sh_interp(bytes, &mut out, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'$'
            && bytes.get(pos.saturating_add(1)).copied() == Some(b'\'')
            && is_word_start(bytes, pos)
        {
            let end = consume_escaped(bytes, pos.saturating_add(1), b'\'');
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'$' && bytes.get(pos.saturating_add(1)).copied() == Some(b'(') {
            // `$(…)` is command text: one code island with paren nesting.
            push(&mut out, RsKind::Code, span_start, pos);
            pos = lex_paren_island(bytes, &mut out, pos);
            span_start = pos;
            continue;
        }
        if byte == b'`' {
            // Backquote command substitution: command text reads as code.
            let end = consume_until_unescaped(bytes, pos.saturating_add(1), b'`');
            push(&mut out, RsKind::Code, span_start, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'\\' {
            pos = pos.saturating_add(2);
            continue;
        }
        pos = pos.saturating_add(1);
    }
    push(&mut out, RsKind::Code, span_start, pos);
    out
}

/// Split `$(…)` / backquote islands out of a `"` string span: islands become
/// `Code`, the rest stays `Str`. Delimiters ride with the island spans.
fn emit_sh_interp(bytes: &[u8], out: &mut Vec<RsSeg>, start: usize, end: usize) {
    let mut chunk = start;
    let mut cur = start;
    while cur < end {
        match bytes.get(cur).copied() {
            Some(b'\\') => {
                cur = cur.saturating_add(2);
            }
            Some(b'$')
                if bytes.get(cur.saturating_add(1)).copied() == Some(b'(') =>
            {
                push(out, RsKind::Str, chunk, cur);
                cur = lex_paren_island(bytes, out, cur);
                chunk = cur;
            }
            Some(b'`') => {
                push(out, RsKind::Str, chunk, cur);
                let close = consume_until_unescaped(bytes, cur.saturating_add(1), b'`');
                push(out, RsKind::Code, cur, close);
                cur = close;
                chunk = cur;
            }
            _ => {
                cur = cur.saturating_add(1);
            }
        }
    }
    push(out, RsKind::Str, chunk, end);
}

/// `$(` island from the `$` at `dollar`: paren-nested, quote-aware-lite,
/// emitted as ONE `Code` span (inner quotes are shell syntax, not strings).
fn lex_paren_island(bytes: &[u8], out: &mut Vec<RsSeg>, dollar: usize) -> usize {
    let mut depth = 0usize;
    let mut cur = dollar;
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'"' || byte == b'\'' || byte == b'`' {
            cur = skip_quoted_island(bytes, cur, byte);
            continue;
        }
        if byte == b'(' {
            depth = depth.saturating_add(1);
        } else if byte == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                push(out, RsKind::Code, dollar, cur.saturating_add(1));
                return cur.saturating_add(1);
            }
        } else if byte == b'\\' {
            cur = cur.saturating_add(1);
        }
        cur = cur.saturating_add(1);
    }
    push(out, RsKind::Code, dollar, cur);
    cur
}

/// Consume to the next unescaped `quote` (shell backquotes: `\` escapes).
fn consume_until_unescaped(bytes: &[u8], mut cur: usize, quote: u8) -> usize {
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

/// True when `pos` begins a word: start of input or preceded by whitespace or
/// a shell operator boundary (POSIX `#`-comment rule, approximated).
fn is_word_start(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    matches!(
        bytes.get(pos.saturating_sub(1)).copied(),
        Some(b' ' | b'\t' | b'\n' | b';' | b'&' | b'|' | b'(' | b')' | b'{' | b'}')
    )
}

#[cfg(test)]
mod tests {
    use super::lex;
    use crate::lex::{RsKind, text};

    fn joined(src: &str) -> String {
        lex(src).iter().map(|s| text(src, s)).collect::<Vec<_>>().join("")
    }

    #[test]
    fn hash_comment_needs_word_start() {
        // POSIX: `a#b` is literal, ` #` opens a comment.
        let src = "echo a#b # real\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let comments: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Comment)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(comments, vec!["# real"]);
    }

    #[test]
    fn single_quotes_are_fully_literal() {
        let src = "echo 'a # b \"c\"'\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert!(!segs.iter().any(|s| s.kind == RsKind::Comment));
        assert_eq!(segs.iter().filter(|s| s.kind == RsKind::Str).count(), 1);
    }

    #[test]
    fn dollar_paren_island_is_code() {
        let src = "echo \"hi $(whoami)\" # done\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("$(whoami)"), "got: {code}");
    }
}
