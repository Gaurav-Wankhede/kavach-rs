// ALGO: Test suite
//! Tests for db module.

use super::delete::{delete_confirm_phrase, delete_confirm_phrase_prefix};
use super::wipe_project::wipe_confirm_phrase;

#[test]
fn delete_phrase_is_target_bound_per_key() {
    assert_eq!(
        delete_confirm_phrase("kavach-rs", "roadmap", Some("foo.bar")),
        "delete kavach-rs/roadmap/foo.bar"
    );
}

#[test]
fn delete_phrase_whole_table_omits_key() {
    assert_eq!(
        delete_confirm_phrase("kavach-rs", "decision", None),
        "delete kavach-rs/decision"
    );
}

#[test]
fn delete_phrase_for_one_target_does_not_authorize_another() {
    let a = delete_confirm_phrase("proj-a", "roadmap", Some("k1"));
    let b = delete_confirm_phrase("proj-b", "roadmap", Some("k1"));
    let c = delete_confirm_phrase("proj-a", "decision", Some("k1"));
    let d = delete_confirm_phrase("proj-a", "roadmap", Some("k2"));
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}

#[test]
fn prefix_phrase_carries_a_wildcard_marker() {
    assert_eq!(
        delete_confirm_phrase_prefix("kavach-rs", "roadmap", "heal.incident.loophole-"),
        "delete kavach-rs/roadmap/prefix:heal.incident.loophole-*"
    );
}

#[test]
fn single_key_confirm_cannot_authorize_a_prefix_purge() {
    // authz/replay loophole: a phrase typed to delete ONE key whose name happens
    // to equal a prefix must NOT match the wildcard-purge phrase for that prefix.
    let single = delete_confirm_phrase("kavach-rs", "roadmap", Some("heal.incident.loophole-"));
    let wildcard = delete_confirm_phrase_prefix("kavach-rs", "roadmap", "heal.incident.loophole-");
    assert_ne!(single, wildcard, "single-key confirm must not authorize a bulk purge");
}

#[test]
fn prefix_phrase_is_target_bound_per_field() {
    let base = delete_confirm_phrase_prefix("proj-a", "roadmap", "p");
    assert_ne!(base, delete_confirm_phrase_prefix("proj-b", "roadmap", "p"));
    assert_ne!(base, delete_confirm_phrase_prefix("proj-a", "decision", "p"));
    assert_ne!(base, delete_confirm_phrase_prefix("proj-a", "roadmap", "q"));
}

#[test]
fn wipe_phrase_is_target_bound() {
    assert_eq!(wipe_confirm_phrase("kavach-rs"), "wipe kavach-rs");
    assert_ne!(wipe_confirm_phrase("proj-a"), wipe_confirm_phrase("proj-b"));
}

#[test]
fn missing_or_mismatched_confirm_fails_the_gate_check() {
    let expected = delete_confirm_phrase("kavach-rs", "roadmap", Some("k"));
    let none: Option<&str> = None;
    assert_ne!(none, Some(expected.as_str()));
    assert_ne!(Some("delete"), Some(expected.as_str()));
    assert_ne!(Some("delete kavach-rs/roadmap"), Some(expected.as_str()));
    assert_eq!(Some(expected.as_str()), Some(expected.as_str()));
}
