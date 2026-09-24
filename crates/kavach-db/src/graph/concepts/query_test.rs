//! Unit tests for the whole-word concept matcher (no DB needed).
use super::{concept_query_words, concept_text_matches};

#[test]
fn single_word_hits_whole_words_only() {
    assert!(concept_text_matches("auth", "login flow", &["auth"]));
    assert!(concept_text_matches("author bio", "profile", &["author"]));
    // Substring is not a hit: "auth" must not match "author".
    assert!(!concept_text_matches("author bio", "profile", &["auth"]));
}

#[test]
fn matching_is_case_insensitive_and_punctuation_tolerant() {
    assert!(concept_text_matches("PASETO Tokens", "auth, scoped.", &["paseto"]));
    assert!(concept_text_matches("x", "trailing comma,", &["comma"]));
    assert!(concept_text_matches("n", "\"quoted\"", &["quoted"]));
}

#[test]
fn multi_word_queries_are_and_not_substring() {
    // Both words anywhere (FTS parity); the contiguous phrase is not required.
    assert!(concept_text_matches("session tokens", "expiry", &["session", "tokens"]));
    assert!(concept_text_matches("tokens", "session scope", &["session", "tokens"]));
    assert!(!concept_text_matches("session log", "expiry", &["session", "tokens"]));
}

#[test]
fn query_words_lex_without_heap_per_word() {
    let words = concept_query_words("  auth   tokens  ");
    assert_eq!(words, vec!["auth", "tokens"]);
    assert!(concept_query_words("   ").is_empty());
}

#[test]
fn json_keys_are_not_searchable_text() {
    // Matching runs on extracted name/desc text, never raw JSON: the word
    // "desc" must not hit just because the properties object has a desc key.
    // (The LIKE this replaces matched `"desc":` key text on every row.)
    assert!(!concept_text_matches("sso", "single sign-on", &["desc"]));
    assert!(!concept_text_matches("sso", "single sign-on", &["tags"]));
}
