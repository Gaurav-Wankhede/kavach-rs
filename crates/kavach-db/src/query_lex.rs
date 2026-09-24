//! Word lexer + micro-AST for operator-supplied query/filter strings.
//!
//! Ruff-truthful design (hand-written cursor lexer, NOT logos/rowan — Ruff
//! itself is a hand-written recursive-descent parser over a hand-written
//! lexer since 0.4.0: `Lexer<'src>` over `&'src str`, `TokenKind` enum,
//! `TextRange` spans, trivia skipped before the parser; SOURCE:
//! <https://astral.sh/blog/ruff-v0.4.0>, `ruff_python_parser/src/lexer.rs`).
//!
//! Every token is a `(TokenKind, TextRange)` pair borrowed from the input:
//! lexing allocates nothing per word (the old `fts5_query` built one `String`
//! per word, N+1 allocations). Keywords (the four board statuses) become typed
//! tokens at lex time, so downstream matching compares `MemoryStatus`
//! discriminants instead of `== "todo"` string scans.

use kavach_types::{MemoryStatus, TextRange};

/// Token kind: either a plain word slice or a board-status keyword recognized
/// at lex time (Ruff's `lex_identifier` keyword table, byte-matched).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TokenKind {
    /// A non-whitespace word; text recovered via [`Lexer::text`], never stored.
    Word,
    /// A canonical board status (`todo`, `in_progress`, `done`, `verified`).
    Status(MemoryStatus),
    /// End of input (never stored in [`QueryAst`]).
    EndOfFile,
}

/// One lexed token: kind + byte span into the source. `Copy` (12 bytes);
/// the word text itself is never materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Token {
    /// What the span is.
    pub kind: TokenKind,
    /// Byte offsets into the lexed source.
    pub range: TextRange,
}

/// Hand-written cursor lexer over borrowed source text.
///
/// Whitespace (Unicode `White_Space`, the same predicate
/// `str::split_whitespace` uses, so tokenization matches the old splitter
/// exactly) is trivia and skipped; every other run of characters is one word.
/// Zero heap allocation while lexing: tokens are returned by value.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Lexer<'src> {
    /// The full borrowed source; tokens are spans into it.
    src: &'src str,
    /// Current byte offset into `src` (always a char boundary).
    pos: usize,
}

impl<'src> Lexer<'src> {
    /// Borrow `src` for lexing, starting at byte 0.
    #[must_use]
    pub const fn new(src: &'src str) -> Self {
        Self { src, pos: 0 }
    }

    /// The borrowed source (for zero-copy [`text`](Self::text) recovery).
    #[must_use]
    pub const fn source(&self) -> &'src str {
        self.src
    }

    /// Zero-copy text of a token: the source slice its range covers.
    #[must_use]
    pub fn text(&self, token: &Token) -> &'src str {
        let start = token.range.start as usize;
        let end = token.range.end as usize;
        self.src.get(start..end).unwrap_or("")
    }

    /// Next token; returns `EndOfFile` (with an empty range at the end) once
    /// exhausted. Always makes progress — never yields twice without consuming.
    pub fn next_token(&mut self) -> Token {
        let bytes = self.src.as_bytes();
        let mut pos = self.pos;
        while pos < bytes.len() && char_at(self.src, pos).is_whitespace() {
            pos = pos.saturating_add(char_len(self.src, pos));
        }
        if pos >= bytes.len() {
            self.pos = bytes.len();
            return Token {
                kind: TokenKind::EndOfFile,
                range: TextRange::new(sat_u32(bytes.len()), sat_u32(bytes.len())),
            };
        }
        let start = pos;
        while pos < bytes.len() && !char_at(self.src, pos).is_whitespace() {
            pos = pos.saturating_add(char_len(self.src, pos));
        }
        self.pos = pos;
        let range = TextRange::new(sat_u32(start), sat_u32(pos));
        let word = self.src.get(start..pos).unwrap_or("");
        Token {
            kind: classify_word(word),
            range,
        }
    }
}

/// Micro-AST: the token stream of one operator-supplied string, with the
/// source retained so every token resolves to a zero-copy slice.
///
/// Only ranges are stored (8 bytes each) — words are never materialized as
/// `String`s. This is the parse tree `think` queries and board filters match
/// against instead of re-scanning quoted statements.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct QueryAst<'src> {
    /// Borrowed source all token ranges point into.
    pub source: &'src str,
    /// Word/status tokens in order (`EndOfFile` excluded).
    pub tokens: Vec<Token>,
}

impl<'src> QueryAst<'src> {
    /// Lex `src` into a token tree (trivia dropped, like Ruff's `TokenSource`).
    #[must_use]
    pub fn parse(src: &'src str) -> Self {
        let mut lexer = Lexer::new(src);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::EndOfFile {
                break;
            }
            tokens.push(token);
        }
        Self {
            source: src,
            tokens,
        }
    }

    /// True when the input held no words (whitespace-only or empty).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Number of word/status tokens.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Zero-copy word texts in order (status keywords included as their slice).
    pub fn words(&self) -> impl Iterator<Item = &'src str> + '_ {
        let src = self.source;
        self.tokens.iter().map(move |t| {
            let start = t.range.start as usize;
            let end = t.range.end as usize;
            src.get(start..end).unwrap_or("")
        })
    }

    /// Board statuses recognized anywhere in the token stream, in order.
    pub fn statuses(&self) -> impl Iterator<Item = (MemoryStatus, TextRange)> + '_ {
        self.tokens.iter().filter_map(|t| match t.kind {
            TokenKind::Status(s) => Some((s, t.range)),
            TokenKind::Word | TokenKind::EndOfFile => None,
        })
    }
}

/// A parsed `--status` filter: the CLI keyword becomes one token at the
/// boundary, and every row match after that is a discriminant compare —
/// never another `== "todo"` string scan.
///
/// `Never` preserves the old exact-match semantics: today an unknown filter
/// word matches zero rows (not "no filter"), and surrounding whitespace also
/// matches nothing. Only the bare canonical keyword selects a lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusPred {
    /// No filter supplied: every row passes.
    Any,
    /// Exactly this lane passes (discriminant compare on `Option<MemoryStatus>`).
    Is(MemoryStatus),
    /// Filter present but not a bare keyword: no row passes.
    Never,
}

impl StatusPred {
    /// Lex the raw `--status` argument into a predicate. `None` (flag absent)
    /// is [`StatusPred::Any`]; anything but the bare canonical keyword —
    /// unknown words, empty input, padded whitespace — is [`StatusPred::Never`].
    #[must_use]
    pub fn from_opt(filter: Option<&str>) -> Self {
        let Some(raw) = filter else {
            return Self::Any;
        };
        let ast = QueryAst::parse(raw);
        if ast.tokens.len() != 1 {
            return Self::Never;
        }
        let Some(only) = ast.tokens.first().copied() else {
            return Self::Never;
        };
        if only.range.len() as usize != raw.len() {
            return Self::Never;
        }
        match only.kind {
            TokenKind::Status(s) => Self::Is(s),
            TokenKind::Word | TokenKind::EndOfFile => Self::Never,
        }
    }

    /// Row match on the typed board status (no string comparison).
    #[must_use]
    pub const fn matches_typed(self, status: Option<MemoryStatus>) -> bool {
        match (self, status) {
            (Self::Any, _) => true,
            (Self::Is(want), Some(got)) => want as u8 == got as u8,
            (Self::Is(_), None) | (Self::Never, _) => false,
        }
    }
}

/// Ruff-style keyword table: length gate then byte match (all four statuses
/// are ASCII, longest is `in_progress` at 11 bytes). Anything else is a word.
fn classify_word(word: &str) -> TokenKind {
    if word.len() > 11 {
        return TokenKind::Word;
    }
    match word.as_bytes() {
        b"todo" => TokenKind::Status(MemoryStatus::Todo),
        b"in_progress" => TokenKind::Status(MemoryStatus::InProgress),
        b"done" => TokenKind::Status(MemoryStatus::Done),
        b"verified" => TokenKind::Status(MemoryStatus::Verified),
        _ => TokenKind::Word,
    }
}

/// Saturating `usize` → `u32` for span storage (queries are never 4 GiB).
fn sat_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// First char at byte offset `pos` (`pos` is always a boundary by construction).
fn char_at(src: &str, pos: usize) -> char {
    src.get(pos..)
        .and_then(|s| s.chars().next())
        .unwrap_or('\0')
}

/// Byte length of the char at offset `pos`.
fn char_len(src: &str, pos: usize) -> usize {
    char_at(src, pos).len_utf8().max(1)
}

#[cfg(test)]
#[path = "query_lex_test.rs"]
mod query_lex_test;
