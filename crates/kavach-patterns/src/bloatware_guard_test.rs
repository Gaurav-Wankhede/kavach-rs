//! Contract for the bloatware (tombstone-comment) guard, written test-FIRST.
//!
//! A tombstone is a COMMENT that documents a removal instead of just removing —
//! the deletion + git history are the record (decision.bloatware.no-tombstone-comments).
//! The signal must be EXACT: only a comment line carrying a removal-marker fires;
//! the same word inside a string literal, an identifier, or live code never does.
//! That exactness is what keeps the false-positive set empty.

use super::detect;
use crate::severity::Severity;

#[test]
fn tombstone_comment_is_p0_blocked() {
    // `//` comment that narrates a removal — the exact bloat the user forbids.
    let v = detect(
        "crates/core/x.rs",
        "// owner-gating abolished 2026-06-20\nfn f() {}",
    );
    assert_eq!(v.len(), 1, "tombstone comment must fire once: {v:?}");
    assert_eq!(v[0].severity, Severity::P0Block);
}

#[test]
fn sql_dash_dash_tombstone_is_blocked() {
    // SQL/`--` comment tombstone (the schema.rs shape) also fires.
    let v = detect(
        "crates/core/m.sql",
        "-- legacy field REMOVED; dropped below for old stores",
    );
    assert_eq!(v.len(), 1, "SQL tombstone comment must fire: {v:?}");
}

#[test]
fn removal_word_in_string_literal_does_not_fire() {
    // "removed" inside a STRING is data, not a tombstone comment — must pass.
    let src = r#"fn msg() -> &'static str { "the record was removed by the user" }"#;
    assert!(
        detect("crates/core/x.rs", src).is_empty(),
        "string literal is not a tombstone"
    );
}

#[test]
fn removal_word_in_identifier_does_not_fire() {
    // A function/var named with a removal word is live code — must pass.
    let src = "fn deprecated_route_handler() {}\nlet abolished_count = 0;";
    assert!(
        detect("crates/core/x.rs", src).is_empty(),
        "identifier is not a tombstone"
    );
}

#[test]
fn ordinary_explanatory_comment_does_not_fire() {
    // A comment that explains live behavior (no removal-marker) must pass.
    let src = "// Umbrella cards are never dispatch targets (counted via children).\nfn f() {}";
    assert!(
        detect("crates/core/x.rs", src).is_empty(),
        "explanatory comment is fine"
    );
}

#[test]
fn non_governed_path_is_exempt() {
    // Match the dedup_guard scope contract: only governed crates are policed.
    let src = "// this field was removed in v2";
    assert!(
        detect("docs/notes.md", src).is_empty(),
        "non-source/non-governed exempt"
    );
}
