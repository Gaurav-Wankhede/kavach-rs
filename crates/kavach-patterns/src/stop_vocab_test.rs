//! Tests for the DB-sourced stop-gate vocabulary. Proves the config-as-data
//! contract: the compiled `Default` IS the fail-open floor, a partial DB override
//! is honored while omitted lists keep the floor, and a malformed blob degrades to
//! the full default (never a panic, never an empty vocab).

use super::{DEFAULT_GAMING_PHRASES, DEFAULT_HANDBACK_PHRASES, DoneGamingVocab};

#[test]
fn default_is_the_compiled_floor() {
    let v = DoneGamingVocab::default();
    assert_eq!(v.gaming_phrases.len(), DEFAULT_GAMING_PHRASES.len());
    assert_eq!(v.handback_phrases.len(), DEFAULT_HANDBACK_PHRASES.len());
    assert!(v.has_gaming_phrase("this is vacuously complete"));
    assert!(v.has_handback_phrase("i am holding for the disk reclaim"));
}

#[test]
fn matching_is_lowercase_substring() {
    let v = DoneGamingVocab::default();
    // Caller lower-cases before matching; the phrases themselves are lower-case.
    assert!(v.has_gaming_phrase("the documentation pass is finished"));
    assert!(!v.has_gaming_phrase("Documentation Pass")); // not pre-lowercased
}

#[test]
fn partial_override_keeps_the_omitted_floor() {
    // Exactly how `done_gaming_vocab_for` deserializes a DB row.
    let row = r#"{"gaming_phrases":["mission accomplished","wrapping up"]}"#;
    let v: DoneGamingVocab = serde_json::from_str(row).expect("valid row");
    // Overridden list is honored...
    assert!(v.has_gaming_phrase("mission accomplished, closing."));
    assert!(!v.has_gaming_phrase("vacuously complete")); // floor replaced, not merged
    // ...and the omitted handback list falls back to the compiled floor.
    assert_eq!(v.handback_phrases.len(), DEFAULT_HANDBACK_PHRASES.len());
    assert!(v.has_handback_phrase("owner must free durable space"));
}

#[test]
fn malformed_blob_degrades_to_default() {
    let v: DoneGamingVocab = serde_json::from_str("{ not valid json").unwrap_or_default();
    assert!(v.has_gaming_phrase("vacuously complete"));
    assert!(v.has_handback_phrase("i'm holding"));
}

#[test]
fn empty_object_is_the_full_default() {
    // `#[serde(default)]` fills BOTH lists when the row is `{}`.
    let v: DoneGamingVocab = serde_json::from_str("{}").expect("empty object");
    assert_eq!(v.gaming_phrases.len(), DEFAULT_GAMING_PHRASES.len());
    assert_eq!(v.handback_phrases.len(), DEFAULT_HANDBACK_PHRASES.len());
}

#[test]
fn explicitly_empty_lists_disable_a_dimension() {
    // An operator can intentionally silence one arm with an empty array — distinct
    // from omission (which keeps the floor). Proves the override is truly load-bearing.
    let row = r#"{"gaming_phrases":[],"handback_phrases":["custom only"]}"#;
    let v: DoneGamingVocab = serde_json::from_str(row).expect("valid row");
    assert!(!v.has_gaming_phrase("vacuously complete")); // silenced
    assert!(v.has_handback_phrase("custom only marker"));
    assert!(!v.has_handback_phrase("i'm holding")); // floor replaced
}
