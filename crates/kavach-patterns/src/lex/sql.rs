//! SQL trivia cursor.
//!
//! `--` line comments, NESTING `/* */`, `'…'` (`''` doubling, no
//! backslash), `E'…'` escapes, `$tag$…$tag$` dollar quotes, `"…"` quoted
//! identifiers (folded into `Str` so `"select"` never reads as a keyword).
//! Source: PostgreSQL lexical docs.
use super::{RsKind, RsSeg, consume_nested, consume_to_eol, push};

/// SQL source cursor.
#[must_use]
pub fn lex(src: &str) -> Vec<RsSeg> {
    let bytes = src.as_bytes();
    let mut out: Vec<RsSeg> = Vec::new();
    let mut pos = 0usize;
    let mut span_start = 0usize;
    while let Some(&byte) = bytes.get(pos) {
        let ahead = bytes.get(pos.saturating_add(1)).copied();
        if byte == b'-' && ahead == Some(b'-') {
            let end = consume_to_eol(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'/' && ahead == Some(b'*') {
            let end = consume_nested(bytes, pos, true);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'\'' || (byte == b'E' || byte == b'e') && ahead == Some(b'\'') {
            // `E'…'`: backslash escapes live; plain `'…'`: only `''` doubling.
            let escapes = byte == b'E' || byte == b'e';
            let content = if escapes { pos.saturating_add(1) } else { pos };
            let end = consume_sql_string(bytes, content, escapes);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'"' {
            let end = consume_sql_dquoted(bytes, pos);
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'$' {
            if let Some(end) = consume_dollar_quote(bytes, pos) {
                push(&mut out, RsKind::Code, span_start, pos);
                push(&mut out, RsKind::Str, pos, end);
                pos = end;
                span_start = pos;
                continue;
            }
            pos = pos.saturating_add(1);
            continue;
        }
        pos = pos.saturating_add(1);
    }
    push(&mut out, RsKind::Code, span_start, pos);
    out
}

/// SQL single-quoted string from the opening quote at `pos`: `''` doubling
/// always closes nothing; backslash escapes only when `escapes` (`E'…'`).
fn consume_sql_string(bytes: &[u8], pos: usize, escapes: bool) -> usize {
    let mut cur = pos.saturating_add(1);
    while let Some(&byte) = bytes.get(cur) {
        if escapes && byte == b'\\' {
            cur = super::adv_esc(bytes, cur);
        } else if byte == b'\'' {
            if bytes.get(cur.saturating_add(1)).copied() == Some(b'\'') {
                cur = cur.saturating_add(2);
            } else {
                return cur.saturating_add(1);
            }
        } else {
            cur = cur.saturating_add(1);
        }
    }
    cur
}

/// SQL double-quoted identifier: `""` doubling, no backslash escapes.
fn consume_sql_dquoted(bytes: &[u8], pos: usize) -> usize {
    let mut cur = pos.saturating_add(1);
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'"' {
            if bytes.get(cur.saturating_add(1)).copied() == Some(b'"') {
                cur = cur.saturating_add(2);
            } else {
                return cur.saturating_add(1);
            }
        } else {
            cur = cur.saturating_add(1);
        }
    }
    cur
}

/// `$tag$…$tag$` dollar quote at `pos` (tag possibly empty); `None` when the
/// `$` does not open one (e.g. `$1` placeholders stay code).
fn consume_dollar_quote(bytes: &[u8], pos: usize) -> Option<usize> {
    let mut cur = pos.saturating_add(1);
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'$' {
            break;
        }
        if !byte.is_ascii_alphanumeric() && byte != b'_' {
            return None;
        }
        cur = cur.saturating_add(1);
    }
    if bytes.get(cur).copied() != Some(b'$') {
        return None;
    }
    let tag_end = cur.saturating_add(1);
    let tag_len = tag_end.saturating_sub(pos);
    let mut close = tag_end;
    while close < bytes.len() {
        if bytes.get(close).copied() == Some(b'$') && tag_matches(bytes, pos, close, tag_len) {
            return Some(close.saturating_add(tag_len));
        }
        close = close.saturating_add(1);
    }
    Some(bytes.len())
}

/// True when `bytes[close..close+tag_len]` repeats the opening `$tag$`.
fn tag_matches(bytes: &[u8], open: usize, close: usize, tag_len: usize) -> bool {
    for i in 0..tag_len {
        if bytes.get(open.saturating_add(i)).copied() != bytes.get(close.saturating_add(i)).copied()
        {
            return false;
        }
    }
    true
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
    fn doubled_quote_does_not_close() {
        let src = "SELECT 'Dianne''s horse'; -- done\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert_eq!(segs.iter().filter(|s| s.kind == RsKind::Str).count(), 1);
        let comments: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Comment)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(comments, vec!["-- done"]);
    }

    #[test]
    fn dollar_quote_holds_quotes_and_backslashes() {
        let src = "SELECT $$Dianne's \\ horse$$; SELECT 1;";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert_eq!(segs.iter().filter(|s| s.kind == RsKind::Str).count(), 1);
    }

    #[test]
    fn block_comment_nests() {
        let src = "/* a /* b */ still */ SELECT 1;";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let kinds: Vec<RsKind> = segs.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, vec![RsKind::Comment, RsKind::Code]);
    }

    #[test]
    fn placeholders_stay_code() {
        let src = "SELECT $1, $2;";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        assert!(!segs.iter().any(|s| s.kind == RsKind::Str));
    }
}
