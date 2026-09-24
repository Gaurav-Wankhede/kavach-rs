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
    /// Script extras: TS/JS hashbang + regex literals; others plain.
    pub script: Script,
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
/// Script extras: TS/JS hashbang (`#!`) + regex literals; other C-likes plain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Script {
    /// No script extras.
    Plain,
    /// TypeScript/JavaScript: hashbang + `/pattern/flags` literals.
    Js,
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
    script: Script::Plain,
    triple: Triple::None,
    dollar_interp: false,
};
pub const C_JAVA: CLike = CLike {
    backquote: Backquote::None,
    char_lit: true,
    block: Block::Flat,
    script: Script::Plain,
    triple: Triple::JavaText,
    dollar_interp: false,
};
pub const C_KOTLIN: CLike = CLike {
    backquote: Backquote::None,
    char_lit: true,
    block: Block::Nested,
    script: Script::Plain,
    triple: Triple::KotlinRaw,
    dollar_interp: true,
};
pub const C_TSJS: CLike = CLike {
    backquote: Backquote::Template,
    char_lit: false,
    block: Block::Flat,
    script: Script::Js,
    triple: Triple::None,
    dollar_interp: false,
};

/// Hashbang prefix: `#!…` at offset 0 lexes as one comment span.
/// Returns the resume `(pos, span_start)` past it (or `(0, 0)`).
fn hashbang_prefix(table: CLike, bytes: &[u8], out: &mut Vec<RsSeg>) -> (usize, usize) {
    if table.script == Script::Js
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
        // TS/JS regex literals: `/` opens one exactly when an operand may
        // follow (never after an expression end). Unterminated/invalid reads
        // fall through as division bytes.
        if byte == b'/'
            && table.script == Script::Js
            && regex_opens(bytes, pos)
            && let Some(end) = try_consume_regex(bytes, pos)
        {
            push(&mut out, RsKind::Code, span_start, pos);
            push(&mut out, RsKind::Str, pos, end);
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

/// Keywords after which a `/` opens a regex (an operand must follow).
const REGEX_KEYWORDS: &[&[u8]] = &[
    b"return",
    b"typeof",
    b"instanceof",
    b"in",
    b"of",
    b"new",
    b"delete",
    b"void",
    b"throw",
    b"case",
    b"do",
    b"else",
    b"yield",
    b"await",
];

/// True when a `/` at `pos` (already known not to open `//`/`/*`) starts a
/// regex literal: the previous significant token cannot end an expression.
///
/// Heuristic (documented approximations): `}` always allows regex (block end;
/// object-literal division like `a={b:1}/2` mislexes — rare); a `/` at a line
/// start after a newline allows regex (ASI statement-start reading; `x\n/y/g`
/// division mislexes — rare); a quote is read as a just-closed string.
fn regex_opens(bytes: &[u8], pos: usize) -> bool {
    let mut cur = pos;
    while cur > 0 {
        match bytes.get(cur.saturating_sub(1)).copied() {
            Some(b' ' | b'\t' | b'\r' | b'\n') => {
                cur = cur.saturating_sub(1);
            }
            _ => break,
        }
    }
    if cur == 0 {
        return true;
    }
    let Some(prev) = bytes.get(cur.saturating_sub(1)).copied() else {
        return true;
    };
    if prev == b')' || prev == b']' {
        return false;
    }
    if prev == b'\'' || prev == b'"' || prev == b'`' {
        return false;
    }
    if prev == b'}' {
        return true;
    }
    if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'$' {
        // Trailing word: keyword → regex, anything else → division.
        return REGEX_KEYWORDS.iter().any(|kw| trailing_word_is(bytes, cur, kw));
    }
    // Operators, openers, commas, semicolons: an operand (regex) follows.
    true
}

/// True when the word ending at `end` (exclusive) equals `word`, starting at
/// a word boundary. Byte-wise (keywords are ASCII).
fn trailing_word_is(bytes: &[u8], end: usize, word: &[u8]) -> bool {
    if end < word.len() {
        return false;
    }
    let start = end.saturating_sub(word.len());
    if start > 0
        && bytes
            .get(start.saturating_sub(1))
            .copied()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
    {
        return false;
    }
    for (i, want) in word.iter().enumerate() {
        if bytes.get(start.saturating_add(i)).copied() != Some(*want) {
            return false;
        }
    }
    true
}

/// Consume a `/pattern/flags` literal from the `/` at `pos` (`//`/`/*`
/// excluded by the caller). Returns the end offset, or `None` when invalid:
/// unterminated, newline outside a `[...]` class, or empty input.
fn try_consume_regex(bytes: &[u8], pos: usize) -> Option<usize> {
    let mut cur = pos.saturating_add(1);
    let mut in_class = false;
    let mut len = 0usize;
    while let Some(&byte) = bytes.get(cur) {
        if byte == b'\\' {
            cur = cur.saturating_add(2);
            len = len.saturating_add(1);
            continue;
        }
        if byte == b'\n' && !in_class {
            return None;
        }
        if byte == b'[' {
            in_class = true;
        } else if byte == b']' && in_class {
            in_class = false;
        } else if byte == b'/' && !in_class {
            if len == 0 {
                return None;
            }
            cur = cur.saturating_add(1);
            while bytes
                .get(cur)
                .copied()
                .is_some_and(|b| b.is_ascii_alphabetic())
            {
                cur = cur.saturating_add(1);
            }
            return Some(cur);
        }
        cur = cur.saturating_add(1);
        len = len.saturating_add(1);
    }
    None
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
        // Comments and regexes inside substitutions must not corrupt brace
        // depth (`// }`, `/a{2}/`). Same rules as the top-level cursor.
        if byte == b'/' {
            match bytes.get(cur.saturating_add(1)).copied() {
                Some(b'/') => {
                    cur = consume_to_eol(bytes, cur);
                    continue;
                }
                Some(b'*') => {
                    cur = consume_nested(bytes, cur, false);
                    continue;
                }
                _ if regex_opens(bytes, cur) => {
                    if let Some(end) = try_consume_regex(bytes, cur) {
                        cur = end;
                        continue;
                    }
                }
                _ => {}
            }
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
    fn ts_block_comment_does_not_nest() {
        let src = "/* a /* b */ const x = 1;";
        assert_eq!(joined(src, C_TSJS), src);
        assert_eq!(
            kinds(src, C_TSJS),
            vec![RsKind::Comment, RsKind::Code]
        );
    }

    #[test]
    fn js_regex_literal_is_str() {
        let src = "const re = /ab+c/gi;\n";
        let segs = lex(src, C_TSJS);
        assert_eq!(joined(src, C_TSJS), src);
        let strs: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Str)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(strs, vec!["/ab+c/gi"]);
    }

    #[test]
    fn js_division_stays_code() {
        let src = "let x = a / b;\n";
        let segs = lex(src, C_TSJS);
        assert_eq!(joined(src, C_TSJS), src);
        assert!(!segs.iter().any(|s| s.kind == RsKind::Str));
    }

    #[test]
    fn js_regex_after_operand_positions() {
        // `(`, `=`, `return` expect an operand: regex. Identifier/`)` end
        // one: division.
        for (src, want_str) in [
            ("foo(/re/);", true),
            ("x = /re/.test(y);", true),
            ("return /re/;", true),
            ("y = a / 2;", false),
            ("f(x) / 2;", false),
        ] {
            let segs = lex(src, C_TSJS);
            assert_eq!(joined(src, C_TSJS), src);
            assert_eq!(
                segs.iter().any(|s| s.kind == RsKind::Str),
                want_str,
                "src={src:?}"
            );
        }
    }

    #[test]
    fn js_regex_class_holds_slash() {
        let src = "const r = /[/]/;\n";
        let segs = lex(src, C_TSJS);
        assert_eq!(joined(src, C_TSJS), src);
        let strs: Vec<&str> = segs
            .iter()
            .filter(|s| s.kind == RsKind::Str)
            .map(|s| text(src, s))
            .collect();
        assert_eq!(strs, vec!["/[/]/"]);
    }

    #[test]
    fn js_template_regex_braces_do_not_break_island() {
        let src = "const s = `${/a{2}/.source}`;\n";
        let segs = lex(src, C_TSJS);
        assert_eq!(joined(src, C_TSJS), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("${/a{2}/.source}"), "got: {code}");
    }

    #[test]
    fn js_template_division_stays_in_island() {
        let src = "const s = `${a / b}`;\n";
        let segs = lex(src, C_TSJS);
        assert_eq!(joined(src, C_TSJS), src);
        let code: String = segs
            .iter()
            .filter(|s| s.kind == RsKind::Code)
            .map(|s| text(src, s))
            .collect::<Vec<_>>()
            .join("");
        assert!(code.contains("${a / b}"), "got: {code}");
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
}
