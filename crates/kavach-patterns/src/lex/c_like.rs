//! C-like trivia cursor: Go, Java, Kotlin, TypeScript/JavaScript.
//!
//! One flag-parameterized cursor over `//`, `/* */`, `"…"`, `'…'`,
//! backquotes. Sources: Go spec, JLS §3, Kotlin spec + strings docs,
//! ECMA-262 + MDN (collected 2026-09).
use super::{
    RsKind, RsSeg, consume_escaped, consume_nested, consume_to_eol, consume_until, push, run_of,
    skip_quoted_island,
};

/// C-like trivia flags: one const per language (the "patterns" table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CLike {
    /// Backquote form: raw string (Go), template with `${}` islands (TS/JS),
    /// or absent (Java/Kotlin use `"""` raws instead).
    pub backquote: Backquote,
    /// `'x'` char/rune literals (Go/Java/Kotlin; TS uses `'` strings).
    pub char_lit: bool,
    /// `/* */` nesting: Kotlin nests, Go/Java/TS close at the first `*/`.
    pub block: Block,
    /// `#!` hashbang comment at offset 0 (TS/JS scripts).
    pub hashbang: bool,
    /// Triple-double-quote raws (`JavaText` needs a trailing newline to open;
    /// `KotlinRaw` opens anywhere, no escapes).
    pub triple: Triple,
    /// `$name` / `${…}` interpolation islands inside strings (Kotlin).
    pub dollar_interp: bool,
}

/// Block-comment nesting: Kotlin nests, Go/Java/TS close at the first `*/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Block {
    /// First `*/` wins (Go/Java/TS per spec).
    Flat,
    /// `/*` nests until balanced (Kotlin per spec).
    Nested,
}
/// Backquote handling per C-like language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Backquote {
    /// No backquote literals.
    None,
    /// Raw string: no escapes, may span lines (Go).
    Raw,
    /// Template: escapes + `${…}` code islands (TS/JS).
    Template,
}

/// Triple-double-quote handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Triple {
    /// No triple-quote form.
    None,
    /// Java text blocks (open requires trailing line terminator).
    JavaText,
    /// Kotlin raw strings (open anywhere, no escapes).
    KotlinRaw,
}

pub const C_GO: CLike = CLike {
    backquote: Backquote::Raw,
    char_lit: true,
    block: Block::Flat,
    hashbang: false,
    triple: Triple::None,
    dollar_interp: false,
};
pub const C_JAVA: CLike = CLike {
    backquote: Backquote::None,
    char_lit: true,
    block: Block::Flat,
    hashbang: false,
    triple: Triple::JavaText,
    dollar_interp: false,
};
pub const C_KOTLIN: CLike = CLike {
    backquote: Backquote::None,
    char_lit: true,
    block: Block::Nested,
    hashbang: false,
    triple: Triple::KotlinRaw,
    dollar_interp: true,
};
pub const C_TSJS: CLike = CLike {
    backquote: Backquote::Template,
    char_lit: false,
    block: Block::Flat,
    hashbang: true,
    triple: Triple::None,
    dollar_interp: false,
};

/// Hashbang prefix: `#!…` at offset 0 lexes as one comment span.
/// Returns the resume `(pos, span_start)` past it (or `(0, 0)`).
fn hashbang_prefix(table: CLike, bytes: &[u8], out: &mut Vec<RsSeg>) -> (usize, usize) {
    if table.hashbang
        && bytes.first().copied() == Some(b'#')
        && bytes.get(1).copied() == Some(b'!')
    {
        let end = consume_to_eol(bytes, 0);
        push(out, RsKind::Comment, 0, end);
        return (end, end);
    }
    (0, 0)
}

/// Shared C-like cursor.
#[must_use]
pub fn lex(src: &str, table: CLike) -> Vec<RsSeg> {
    let bytes = src.as_bytes();
    let mut out: Vec<RsSeg> = Vec::new();
    // `#!` hashbang: comment iff it opens offset 0.
    let (mut pos, mut span_start) = hashbang_prefix(table, bytes, &mut out);
    while let Some(&byte) = bytes.get(pos) {
        let ahead = bytes.get(pos.saturating_add(1)).copied();
        if byte == b'/' && (ahead == Some(b'/') || ahead == Some(b'*')) {
            let end = if ahead == Some(b'/') {
                consume_to_eol(bytes, pos)
            } else {
                consume_nested(bytes, pos, table.block == Block::Nested)
            };
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Comment, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'"' {
            // Triple-quote raws before plain strings.
            if table.triple == Triple::KotlinRaw && run_of(bytes, pos, b'"', 3) {
                let end = consume_until(bytes, pos.saturating_add(3), b'"', 3);
                push(&mut out, RsKind::Code, span_start, pos);
                emit_str_interp(bytes, &mut out, pos, end, table.dollar_interp);
                pos = end;
                span_start = pos;
                continue;
            }
            if table.triple == Triple::JavaText && is_java_text_open(bytes, pos) {
                let end = consume_until(bytes, pos.saturating_add(3), b'"', 3);
                push(&mut out, RsKind::Code, span_start, pos);
                // Java has no `$` interpolation: one opaque span.
                push(&mut out, RsKind::Str, pos, end);
                pos = end;
                span_start = pos;
                continue;
            }
            let end = consume_escaped(bytes, pos, b'"');
            push(&mut out, RsKind::Code, span_start, pos);
            emit_str_interp(bytes, &mut out, pos, end, table.dollar_interp);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'\'' {
            if table.char_lit {
                // Go runes / Java / Kotlin chars (no lifetimes in these
                // languages, so `'x'`-shaped spans are always literals).
                if let Some(end) = char_end(bytes, pos) {
                    push(&mut out, RsKind::Code, span_start, pos);
                    push(&mut out, RsKind::Str, pos, end);
                    pos = end;
                    span_start = pos;
                    continue;
                }
                pos = pos.saturating_add(1);
                continue;
            }
            // TS/JS single-quoted string (escapes, no raw newline).
            let end = consume_escaped(bytes, pos, b'\'');
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
            pos = end;
            span_start = pos;
            continue;
        }
        if byte == b'`' {
            match table.backquote {
                Backquote::None => {
                    pos = pos.saturating_add(1);
                    continue;
                }
                Backquote::Raw => {
                    let end = consume_until(bytes, pos.saturating_add(1), b'`', 1);
                    push(&mut out, RsKind::Code, span_start, pos);
                    push(&mut out, RsKind::Str, pos, end);
                    pos = end;
                    span_start = pos;
                    continue;
                }
                Backquote::Template => {
                    pos = lex_template(bytes, &mut out, span_start, pos);
                    span_start = pos;
                    continue;
                }
            }
        }
        pos = pos.saturating_add(1);
    }
    push(&mut out, RsKind::Code, span_start, pos);
    out
}

/// Emit a string span, splitting `$name` / `${…}` islands into `Code` when
/// `interp` holds (Kotlin); otherwise one opaque `Str` span.
fn emit_str_interp(bytes: &[u8], out: &mut Vec<RsSeg>, start: usize, end: usize, interp: bool) {
    if !interp {
        push(out, RsKind::Str, start, end);
        return;
    }
    let mut chunk = start;
    let mut cur = start;
    while cur < end {
        if bytes.get(cur).copied() == Some(b'$') {
            match bytes.get(cur.saturating_add(1)).copied() {
                Some(b'{') => {
                    push(out, RsKind::Str, chunk, cur);
                    cur = lex_brace_island(bytes, out, cur.saturating_add(2));
                    chunk = cur;
                    continue;
                }
                Some(next) if next.is_ascii_alphabetic() || next == b'_' => {
                    push(out, RsKind::Str, chunk, cur);
                    let name_end = consume_ident(bytes, cur.saturating_add(1));
                    push(out, RsKind::Code, cur, name_end);
                    cur = name_end;
                    chunk = cur;
                    continue;
                }
                _ => {}
            }
        }
        cur = cur.saturating_add(1);
    }
    push(out, RsKind::Str, chunk, end);
}

/// Consume an identifier tail (letters/digits/`_`) from `cur`.
fn consume_ident(bytes: &[u8], mut cur: usize) -> usize {
    while bytes
        .get(cur)
        .copied()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        cur = cur.saturating_add(1);
    }
    cur
}

/// Template literal starting at backquote `pos`: emits head `Str`, `${…}`
/// islands as `Code` (brace-nested), tail `Str` (delimiters ride with the
/// adjacent `Str` chunks so spans tile losslessly); returns the end offset.
fn lex_template(bytes: &[u8], out: &mut Vec<RsSeg>, span_start: usize, pos: usize) -> usize {
    push(out, RsKind::Code, span_start, pos);
    let mut cur = pos.saturating_add(1);
    let mut chunk_start = pos;
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\\' {
            cur = cur.saturating_add(2);
            continue;
        }
        if byte == b'`' {
            push(out, RsKind::Str, chunk_start, cur.saturating_add(1));
            return cur.saturating_add(1);
        }
        if byte == b'$' && bytes.get(cur.saturating_add(1)).copied() == Some(b'{') {
            push(out, RsKind::Str, chunk_start, cur);
            cur = lex_brace_island(bytes, out, cur.saturating_add(2));
            chunk_start = cur;
            continue;
        }
        cur = cur.saturating_add(1);
    }
    push(out, RsKind::Str, chunk_start, cur);
    cur
}

/// `${` island starting after the brace: balanced `{}` (nesting-aware,
/// quote-skipping) emitted as `Code`; returns offset past `}`.
fn lex_brace_island(bytes: &[u8], out: &mut Vec<RsSeg>, mut cur: usize) -> usize {
    let start = cur.saturating_sub(2);
    let mut depth = 1usize;
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'"' || byte == b'\'' || byte == b'`' {
            cur = skip_quoted_island(bytes, cur, byte);
            continue;
        }
        if byte == b'{' {
            depth = depth.saturating_add(1);
        } else if byte == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                push(out, RsKind::Code, start, cur.saturating_add(1));
                return cur.saturating_add(1);
            }
        }
        cur = cur.saturating_add(1);
    }
    push(out, RsKind::Code, start, cur);
    cur
}

/// `'…'` end for Go runes / Java / Kotlin chars: same-line, no-whitespace
/// heuristic (no lifetimes exist there, so this is exact in practice).
fn char_end(bytes: &[u8], pos: usize) -> Option<usize> {
    let mut cur = pos.saturating_add(1);
    let mut len = 0usize;
    while cur < bytes.len() && len < 12 {
        match bytes.get(cur).copied() {
            None | Some(b'\n') => return None,
            Some(b'\'') => return if len == 0 { None } else { Some(cur.saturating_add(1)) },
            Some(b'\\') => {
                cur = cur.saturating_add(2);
                len = len.saturating_add(1);
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

/// Java `"""` text-block open: triple quote + only spaces/tabs to newline.
fn is_java_text_open(bytes: &[u8], pos: usize) -> bool {
    if !run_of(bytes, pos, b'"', 3) {
        return false;
    }
    let mut cur = pos.saturating_add(3);
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\n' {
            return true;
        }
        if byte != b' ' && byte != b'\t' && byte != b'\x0C' {
            return false;
        }
        cur = cur.saturating_add(1);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{C_GO, C_JAVA, C_KOTLIN, C_TSJS, lex};
    use crate::lex::{RsKind, text};

    fn kinds(src: &str, table: super::CLike) -> Vec<RsKind> {
        lex(src, table).iter().map(|s| s.kind).collect()
    }

    fn joined(src: &str, table: super::CLike) -> String {
        lex(src, table)
            .iter()
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn go_raw_string_holds_comment_lookalikes() {
        // go.dev spec: backquote raws span lines, backslashes inert.
        let src = "package main\nvar s = `// not code\n\\n still raw`;\n// real\n";
        let segs = lex(src, C_GO);
        assert_eq!(joined(src, C_GO), src);
        let comments: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Comment)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(comments, vec!["// real"]);
    }

    #[test]
    fn go_block_comment_does_not_nest() {
        // First `*/` wins per spec.
        let src = "/* a /* b */ let x = 1;";
        assert_eq!(joined(src, C_GO), src);
        assert_eq!(
            kinds(src, C_GO),
            vec![RsKind::Comment, RsKind::Code]
        );
    }

    #[test]
    fn java_text_block_is_one_str_span() {
        let src = "String s = \"\"\"\nhello // world\n\"\"\";\nint x = 1;";
        let segs = lex(src, C_JAVA);
        assert_eq!(joined(src, C_JAVA), src);
        assert!(!segs.iter().any(|s| s.kind == RsKind::Comment));
        assert_eq!(
            segs.iter().filter(|s| s.kind == RsKind::Str).count(),
            1
        );
    }

    #[test]
    fn kotlin_block_comment_nests() {
        let src = "/* outer /* inner */ still */ val x = 1";
        assert_eq!(joined(src, C_KOTLIN), src);
        assert_eq!(
            kinds(src, C_KOTLIN),
            vec![RsKind::Comment, RsKind::Code]
        );
    }

    #[test]
    fn kotlin_dollar_template_is_code() {
        let src = "val s = \"hello $name and ${a + b}\";";
        let segs = lex(src, C_KOTLIN);
        assert_eq!(joined(src, C_KOTLIN), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("$name"), "got: {code}");
        assert!(code.contains("${a + b}"), "got: {code}");
    }

    #[test]
    fn ts_template_islands_are_code() {
        let src = "const s = `hi ${user.name}!`; // bye\n";
        let segs = lex(src, C_TSJS);
        assert_eq!(joined(src, C_TSJS), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("${user.name}"), "got: {code}");
        assert!(!segs
            .iter()
            .any(|s| s.kind == RsKind::Comment && text(src, s) != "// bye"));
    }

    #[test]
    fn ts_block_comment_does_not_nest() {
        let src = "/* a /* b */ const x = 1;";
        assert_eq!(
            kinds(src, C_TSJS),
            vec![RsKind::Comment, RsKind::Code]
        );
    }
}
