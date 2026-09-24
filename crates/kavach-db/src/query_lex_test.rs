//! Regression tests for the word lexer + micro-AST (Ruff-style refinement of
//! the `think`/board string-matching paths).
use super::{Lexer, QueryAst, StatusPred, TokenKind};
use kavach_types::MemoryStatus;

fn kinds(src: &str) -> Vec<TokenKind> {
    QueryAst::parse(src).tokens.iter().map(|t| t.kind).collect()
}

#[test]
fn words_split_on_whitespace_with_lossless_spans() {
    let ast = QueryAst::parse("  hello   world  ");
    assert_eq!(ast.len(), 2);
    let words: Vec<&str> = ast.words().collect();
    assert_eq!(words, vec!["hello", "world"]);
    // Spans slice back to the exact words (zero-copy recovery).
    let lexer = Lexer::new("  hello   world  ");
    assert_eq!(lexer.text(&ast.tokens[0]), "hello");
    assert_eq!(lexer.text(&ast.tokens[1]), "world");
    assert_eq!(ast.tokens[0].range.start, 2);
    assert_eq!(ast.tokens[1].range.start, 10);
}

#[test]
fn status_keywords_become_typed_tokens() {
    assert_eq!(
        kinds("todo in_progress done verified"),
        vec![
            TokenKind::Status(MemoryStatus::Todo),
            TokenKind::Status(MemoryStatus::InProgress),
            TokenKind::Status(MemoryStatus::Done),
            TokenKind::Status(MemoryStatus::Verified),
        ]
    );
    let ast = QueryAst::parse("todo in_progress done verified");
    let statuses: Vec<MemoryStatus> = ast.statuses().map(|(s, _)| s).collect();
    assert_eq!(
        statuses,
        vec![
            MemoryStatus::Todo,
            MemoryStatus::InProgress,
            MemoryStatus::Done,
            MemoryStatus::Verified,
        ]
    );
}

#[test]
fn keyword_match_is_exact_and_case_sensitive() {
    // Prefixes, suffixes, case variants, and overlong words stay plain words —
    // the old `== "todo"` scans were exact, and the table preserves that.
    for raw in ["Todo", "TODO", "Done", "DONE", "todos", "todo!", "in-progress", "done2"] {
        assert_eq!(kinds(raw), vec![TokenKind::Word], "raw={raw:?}");
    }
}

#[test]
fn empty_and_whitespace_only_yield_no_tokens() {
    for raw in ["", "   ", "\t\n "] {
        let ast = QueryAst::parse(raw);
        assert!(ast.is_empty(), "raw={raw:?}");
        assert_eq!(ast.len(), 0);
    }
    let mut lexer = Lexer::new("   ");
    assert_eq!(lexer.next_token().kind, TokenKind::EndOfFile);
}

#[test]
fn unicode_words_split_on_unicode_whitespace() {
    // NBSP (U+00A0) is White_Space: same boundary `split_whitespace` uses.
    let ast = QueryAst::parse("h\u{e9}llo\u{a0}world w\u{f6}rk");
    let words: Vec<&str> = ast.words().collect();
    assert_eq!(words, vec!["h\u{e9}llo", "world", "w\u{f6}rk"]);
    // Byte offsets stay on char boundaries for multibyte text.
    for t in &ast.tokens {
        let slice = ast.source.get(t.range.start as usize..t.range.end as usize);
        assert!(slice.is_some(), "span on boundary: {t:?}");
    }
}

#[test]
fn quotes_stay_inside_words_for_fts_parity() {
    // The FTS escaper quotes every whitespace-separated term, so `"` must not
    // be a token boundary (identical term stream to the old splitter).
    let words: Vec<&str> = QueryAst::parse("foo \"bar\" baz").words().collect();
    assert_eq!(words, vec!["foo", "\"bar\"", "baz"]);
}

#[test]
fn token_is_stack_only_no_heap_per_word() {
    fn is_copy<T: Copy>() {}
    // Bounds, not exact sizes: layout is compiler-chosen, but a token must
    // stay a small stack value (kind + one 8-byte span), never a heap handle.
    assert!(size_of::<TokenKind>() <= 8);
    assert!(size_of::<super::Token>() <= 16);
    // Copy: moving tokens never allocates or drops.
    is_copy::<TokenKind>();
    is_copy::<super::Token>();
}

#[test]
fn status_pred_absent_filter_matches_everything() {
    let pred = StatusPred::from_opt(None);
    assert_eq!(pred, StatusPred::Any);
    assert!(pred.matches_typed(Some(MemoryStatus::Todo)));
    assert!(pred.matches_typed(None));
}

#[test]
fn status_pred_bare_keyword_selects_lane() {
    for (raw, want) in [
        ("todo", MemoryStatus::Todo),
        ("in_progress", MemoryStatus::InProgress),
        ("done", MemoryStatus::Done),
        ("verified", MemoryStatus::Verified),
    ] {
        let pred = StatusPred::from_opt(Some(raw));
        assert_eq!(pred, StatusPred::Is(want), "raw={raw:?}");
        assert!(pred.matches_typed(Some(want)));
        assert!(!pred.matches_typed(Some(MemoryStatus::Todo)) || want == MemoryStatus::Todo);
        assert!(!pred.matches_typed(None));
    }
}

#[test]
fn status_pred_garbage_matches_nothing_preserving_exact_semantics() {
    // Today `--status garbage` (or padded/empty input) matches zero rows —
    // `Never`, not "no filter".
    for raw in ["garbage", "", "   ", " todo ", "todo todo", "Done", "TODO"] {
        let pred = StatusPred::from_opt(Some(raw));
        assert_eq!(pred, StatusPred::Never, "raw={raw:?}");
        assert!(!pred.matches_typed(Some(MemoryStatus::Todo)));
        assert!(!pred.matches_typed(None));
    }
}
