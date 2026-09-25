//! Shell trivia cursor (sh/bash/zsh).
//!
//! `#` comments only at word start (POSIX: `a#b` is literal); `'...'`
//! fully literal; `"…"` + `$'…'`; `$(…)` nests as code islands; backquote
//! runs are command text (`Code`); `<<[-]WORD` heredoc bodies are `Str`;
//! `\` escapes + continuation.
use super::{
    RsKind, RsSeg, consume_escaped, consume_to_eol, consume_until, push, skip_quoted_island,
};

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
            pos = lex_paren_island(bytes, &mut out, pos, bytes.len());
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
        if byte == b'<' && bytes.get(pos.saturating_add(1)).copied() == Some(b'<') {
            // Heredoc `<<[-]WORD` (not `<<<` herestring): the operator line
            // stays code, the body through the delimiter line becomes `Str`
            // (prose/data, expansion or not — detectors must not read it).
            if bytes.get(pos.saturating_add(2)).copied() == Some(b'<') {
                pos = pos.saturating_add(2);
                continue;
            }
            if let Some(end) = lex_heredoc(bytes, &mut out, span_start, pos) {
                pos = end;
                span_start = pos;
                continue;
            }
            pos = pos.saturating_add(2);
            continue;
        }
        if byte == b'\\' {
            pos = super::adv_esc(bytes, pos);
            continue;
        }
        pos = pos.saturating_add(1);
    }
    push(&mut out, RsKind::Code, span_start, pos);
    out
}

/// Split `$(…)` / backquote islands out of a `"` string span: islands become
/// `Code`, the rest stays `Str`. Delimiters ride with the island spans.
/// Islands are bounded by `end` (a consumer that cannot close inside the
/// string stops there — the outer cursor resumes after it, so no byte is
/// ever scanned twice).
fn emit_sh_interp(bytes: &[u8], out: &mut Vec<RsSeg>, start: usize, end: usize) {
    let mut chunk = start;
    let mut cur = start;
    while cur < end {
        match bytes.get(cur).copied() {
            Some(b'\\') => {
                cur = super::adv_esc(bytes, cur).min(end);
            }
            Some(b'$') if bytes.get(cur.saturating_add(1)).copied() == Some(b'(') => {
                push(out, RsKind::Str, chunk, cur);
                cur = lex_paren_island(bytes, out, cur, end);
                chunk = cur;
            }
            Some(b'`') => {
                push(out, RsKind::Str, chunk, cur);
                let close = consume_until_unescaped(bytes, cur.saturating_add(1), b'`')
                    .min(end);
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

/// `$(` island from the `$` at `dollar`, bounded by `bound`: paren-nested,
/// quote-aware-lite. Emitted as `Code` spans, except heredoc bodies inside,
/// which become `Str` (a `)` in a body must neither close the island nor read
/// as code). Never scans past `bound`, so an enclosing span's end is always
/// respected (no double-scan overlap).
fn lex_paren_island(bytes: &[u8], out: &mut Vec<RsSeg>, dollar: usize, bound: usize) -> usize {
    let mut depth = 0usize;
    let mut cur = dollar;
    let mut seg_start = dollar;
    while cur < bound {
        let Some(&byte) = bytes.get(cur) else {
            break;
        };
        if byte == b'"' || byte == b'\'' || byte == b'`' {
            cur = skip_quoted_island(bytes, cur, byte).min(bound);
            continue;
        }
        if byte == b'<'
            && bytes.get(cur.saturating_add(1)).copied() == Some(b'<')
            && bytes.get(cur.saturating_add(2)).copied() != Some(b'<')
            && let Some((dash, delim, body)) = parse_heredoc_header(bytes, cur)
            && body < bound
            && let Some(end) = heredoc_end(bytes, body, delim, dash)
        {
            // Clamp to the island bound: an unterminated heredoc cannot
            // swallow the enclosing span's end.
            let end = end.min(bound);
            push(out, RsKind::Code, seg_start, body.min(end));
            push(out, RsKind::Str, body.min(end), end);
            seg_start = end;
            cur = end;
            continue;
        }
        if byte == b'(' {
            depth = depth.saturating_add(1);
        } else if byte == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                push(out, RsKind::Code, seg_start, cur.saturating_add(1));
                return cur.saturating_add(1);
            }
        } else if byte == b'\\' {
            cur = super::adv_esc(bytes, cur).min(bound);
            continue;
        }
        cur = cur.saturating_add(1);
    }
    push(out, RsKind::Code, seg_start, cur);
    cur
}

/// Heredoc at `pos` (first `<` of `<<`, in code): parse `<<[-]WORD` on this
/// line; on success emit the operator line as `Code` and the body through
/// the delimiter line as `Str`, returning the resume offset. `None` when no
/// delimiter word follows on this line or no terminator exists (plain
/// redirections fall through as code).
fn lex_heredoc(bytes: &[u8], out: &mut Vec<RsSeg>, span_start: usize, pos: usize) -> Option<usize> {
    let (dash, delim, body) = parse_heredoc_header(bytes, pos)?;
    let end = heredoc_end(bytes, body, delim, dash)?;
    push(out, RsKind::Code, span_start, body);
    push(out, RsKind::Str, body, end);
    Some(end)
}

/// Parse `<<[-]WORD` at `pos`: `(dash, delimiter, body_start)`. The delimiter
/// word must sit on the same line; quotes stripped one layer. `None` for
/// `<<<`, empty words, or EOL-first (plain redirections).
fn parse_heredoc_header(bytes: &[u8], pos: usize) -> Option<(bool, &[u8], usize)> {
    let mut cur = pos.saturating_add(2);
    // Optional `-` (leading tabs stripped on terminator lines).
    let dash = bytes.get(cur).copied() == Some(b'-');
    if dash {
        cur = cur.saturating_add(1);
    }
    while matches!(bytes.get(cur).copied(), Some(b' ' | b'\t')) {
        cur = cur.saturating_add(1);
    }
    // Delimiter word on this same line (quoted or bare); empty/EOL → no heredoc.
    let word_start = cur;
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\n'
            || byte == b' '
            || byte == b'\t'
            || matches!(byte, b';' | b'&' | b'|' | b'<' | b'>' | b'(' | b')')
        {
            break;
        }
        cur = cur.saturating_add(1);
    }
    if cur == word_start {
        return None;
    }
    let mut delim = bytes.get(word_start..cur)?;
    // One layer of matching outer quotes (`"EOF"`, `'EOF'`).
    for quote in *b"\"'" {
        if delim.len() >= 2
            && delim.first().copied() == Some(quote)
            && delim.last().copied() == Some(quote)
        {
            let inner = delim.get(1..delim.len().saturating_sub(1))?;
            delim = inner;
            break;
        }
    }
    if delim.is_empty() {
        return None;
    }
    // Body starts past the operator line's newline.
    let mut body = cur;
    while bytes.get(body).copied().is_some_and(|b| b != b'\n') {
        body = body.saturating_add(1);
    }
    if bytes.get(body).copied() != Some(b'\n') {
        return None;
    }
    Some((dash, delim, body.saturating_add(1)))
}

/// Scan line by line from `body` for the delimiter-only terminator (tabs
/// allowed first when `dash`); returns the offset past its newline, or `None`
/// when unterminated.
fn heredoc_end(bytes: &[u8], body: usize, delim: &[u8], dash: bool) -> Option<usize> {
    let mut line = body;
    loop {
        let mut line_end = line;
        while bytes.get(line_end).copied().is_some_and(|b| b != b'\n') {
            line_end = line_end.saturating_add(1);
        }
        let mut text = bytes.get(line..line_end)?;
        while let [rest @ .., b'\r'] = text {
            text = rest;
        }
        let mut cmp = text;
        if dash {
            while let [b'\t', rest @ ..] = cmp {
                cmp = rest;
            }
        }
        if cmp == delim {
            return Some(if bytes.get(line_end).copied() == Some(b'\n') {
                line_end.saturating_add(1)
            } else {
                line_end
            });
        }
        if bytes.get(line_end).copied() != Some(b'\n') {
            return None;
        }
        line = line_end.saturating_add(1);
    }
}

/// Consume to the next unescaped `quote` (shell backquotes: `\` escapes).
fn consume_until_unescaped(bytes: &[u8], mut cur: usize, quote: u8) -> usize {
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\\' {
            cur = super::adv_esc(bytes, cur);
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
        lex(src)
            .iter()
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("")
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

    #[test]
    fn heredoc_body_is_str_not_code() {
        let src = "cat <<EOF\nhello $(not-code)\n# not comment\nEOF\necho done\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(!code.contains("not-code"), "got: {code}");
        assert!(!code.contains("not comment"), "got: {code}");
        assert!(code.contains("echo done"), "got: {code}");
    }

    #[test]
    fn heredoc_dash_strips_tabs_on_terminator() {
        let src = "cat <<-EOF\n\tbody\n\tEOF\necho done\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(!code.contains("body"), "got: {code}");
        assert!(code.contains("echo done"), "got: {code}");
    }

    #[test]
    fn herestring_is_not_a_heredoc() {
        // `<<<` takes one word: no body tracking past the line.
        let src = "cat <<< hello\n echo done\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("hello"), "got: {code}");
    }

    #[test]
    fn unterminated_heredoc_falls_back_to_code() {
        let src = "cat <<EOF\nbody with no terminator\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("body with no terminator"), "got: {code}");
    }

    #[test]
    fn island_overrun_never_double_scans() {
        // Fuzzer-found: an island consumer running past its enclosing string
        // end, then the outer cursor rescanning those bytes (overlap).
        for src in ["a`\"`\"&", "x \"$(a{b\" y", "q=\"`u\"v"] {
            let segs = lex(src);
            let mut prev = 0usize;
            for seg in &segs {
                let (start, end) = (seg.start as usize, seg.end as usize);
                assert!(start >= prev, "overlap in {src:?}: {segs:?}");
                prev = end;
            }
            assert_eq!(prev, src.len(), "gap in {src:?}");
        }
    }

    #[test]
    fn heredoc_inside_dollar_paren_keeps_depth() {
        // A `)` inside the heredoc body must not close the `$(…)` island.
        let src = "x=$(cat <<EOF\nnot code (here)\nEOF\n)\n";
        let segs = lex(src);
        assert_eq!(joined(src), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("$(cat"), "got: {code}");
        assert!(!code.contains("not code"), "got: {code}");
    }
}
