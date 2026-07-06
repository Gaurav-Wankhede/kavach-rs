use super::*;

#[test]
fn ledger_key_is_stable_for_same_sample() {
    let a = ledger_key("permission", "should i proceed");
    let b = ledger_key("permission", "SHOULD I PROCEED");
    assert_eq!(a, b, "case-insensitive key for stable dedup");
}

#[test]
fn ledger_key_differs_per_gate() {
    let a = ledger_key("permission", "should i proceed");
    let b = ledger_key("deferral", "should i proceed");
    assert_ne!(a, b);
}

#[test]
fn ledger_key_sanitizes_gate_name() {
    let k = ledger_key("permission/seeking phase-2", "x");
    assert!(!k.contains('/'), "slashes stripped");
    assert!(!k.contains(' '), "spaces stripped");
    assert!(!k.contains('-'), "dashes stripped");
}

#[test]
fn record_outcome_displays_as_key() {
    let o = RecordOutcome {
        key: "mistake.g.abcd1234".to_owned(),
        persisted: true,
        error: None,
    };
    assert_eq!(o.to_string(), "mistake.g.abcd1234");
}

#[test]
fn record_outcome_failure_carries_error() {
    let o = RecordOutcome {
        key: "mistake.g.abcd1234".to_owned(),
        persisted: false,
        error: Some("db write exit=1".to_owned()),
    };
    assert!(!o.persisted);
    assert!(o.error.is_some());
}

#[test]
fn truncate_keeps_short_strings() {
    assert_eq!(truncate("hi", 10), "hi");
}

#[test]
fn truncate_caps_long_strings_with_ellipsis() {
    let out = truncate("0123456789ABCDEF", 5);
    assert!(out.ends_with('…'));
    assert!(out.chars().count() == 6);
}

/// Pins the TOCTOU-fix invariant: the write-intent decision is made by the
/// write attempt itself (try --update-key, fall back to --new on not-found),
/// never by a separate probe-then-branch read that can go stale under a
/// concurrent writer. This function only asserts the shape of that decision.
#[test]
fn write_intent_falls_back_from_update_to_new_on_not_found() {
    let not_found_stderr = "error: key not found in project+category";
    assert!(is_key_not_found(not_found_stderr));
    assert!(!is_key_not_found("error: some other failure"));
}
