//! Unit tests for the Rust trivia cursor (frozen behavior).
use super::{code_words, segments};
use crate::lex::{RsKind, line_no, text};

fn kinds(src: &str) -> Vec<RsKind> {
    segments(src).iter().map(|s| s.kind).collect()
}

fn joined(src: &str) -> String {
    // Lossless: spans tile the input exactly.
    let segs = segments(src);
    let mut out = String::new();
    for s in &segs {
        out.push_str(text(src, s));
    }
    out
}

#[test]
fn plain_code_is_one_span_and_lossless() {
    let src = "fn main() { let x = 1; }\n";
    assert_eq!(kinds(src), vec![RsKind::Code]);
    assert_eq!(joined(src), src);
}

#[test]
fn line_comment_splits_code_spans() {
    let src = "let x = 1; // set x\nlet y = 2;\n";
    let segs = segments(src);
    assert!(segs.iter().any(|s| s.kind == RsKind::Comment));
    assert_eq!(joined(src), src);
    let comments: Vec<&str> = segs
        .iter()
        .filter(|s| s.kind == RsKind::Comment)
        .map(|s| text(src, s))
        .collect();
    assert_eq!(comments, vec!["// set x"]);
}

#[test]
fn block_comments_nest_and_stay_comment() {
    let src = "let x = /* outer /* inner */ still */ 1;";
    let segs = segments(src);
    assert_eq!(joined(src), src);
    let kinds: Vec<RsKind> = segs.iter().map(|s| s.kind).collect();
    assert_eq!(kinds, vec![RsKind::Code, RsKind::Comment, RsKind::Code]);
    assert_eq!(text(src, &segs[1]), "/* outer /* inner */ still */");
}

#[test]
fn string_content_is_opaque_to_code_matchers() {
    // `//` and `fn` inside a string must not lex as comment/code keywords.
    let src = "let s = \"not // a comment, fn fake()\";\nfn real() {}\n";
    let segs = segments(src);
    assert_eq!(joined(src), src);
    assert!(!segs.iter().any(|s| s.kind == RsKind::Comment));
    let words = code_words(src, &segs);
    assert!(words.contains(&"real"));
    assert!(!words.contains(&"fake"));
}

#[test]
fn raw_strings_with_hashes_survive() {
    let src = "let s = r#\"say \"hi\" // ok\"#;\nlet t = 1;";
    let segs = segments(src);
    assert_eq!(joined(src), src);
    assert!(!segs.iter().any(|s| s.kind == RsKind::Comment));
    let strs: Vec<&str> = segs
        .iter()
        .filter(|s| s.kind == RsKind::Str)
        .map(|s| text(src, s))
        .collect();
    assert_eq!(strs, vec!["r#\"say \"hi\" // ok\"#"]);
}

#[test]
fn char_literal_is_str_but_lifetime_is_code() {
    let src = "let c = 'a';\nfn f(x: &'static str) {}\n";
    let segs = segments(src);
    assert_eq!(joined(src), src);
    let strs: Vec<&str> = segs
        .iter()
        .filter(|s| s.kind == RsKind::Str)
        .map(|s| text(src, s))
        .collect();
    assert_eq!(strs, vec!["'a'"]);
    // The lifetime quote stays code (no string opened).
    assert!(
        segs.iter()
            .any(|s| s.kind == RsKind::Code && text(src, s).contains("'static"))
    );
}

#[test]
fn escaped_quote_does_not_end_string() {
    let src = "let s = \"a \\\" // still string\";\nlet t = 1; // comment\n";
    let segs = segments(src);
    assert_eq!(joined(src), src);
    let comments: Vec<&str> = segs
        .iter()
        .filter(|s| s.kind == RsKind::Comment)
        .map(|s| text(src, s))
        .collect();
    assert_eq!(comments, vec!["// comment"]);
}

#[test]
fn unicode_text_passes_through_untouched() {
    let src = "// héllo wörld\nlet s = \"日本語\"; // ok\n";
    assert_eq!(joined(src), src);
    assert_eq!(
        kinds(src).iter().filter(|k| **k == RsKind::Comment).count(),
        2
    );
}

#[test]
fn line_numbers_count_newlines() {
    let src = "a\nb\nc";
    assert_eq!(line_no(src, 0), 1);
    assert_eq!(line_no(src, 2), 2);
    assert_eq!(line_no(src, 4), 3);
}

#[test]
fn code_words_skip_strings_and_comments() {
    let src = "// fn decoy\nlet s = \"struct Fake\";\nstruct Real {}\n";
    let segs = segments(src);
    let words = code_words(src, &segs);
    assert!(words.contains(&"Real"));
    assert!(!words.contains(&"Fake"));
    assert!(!words.contains(&"decoy"));
    assert!(words.contains(&"struct"));
}
