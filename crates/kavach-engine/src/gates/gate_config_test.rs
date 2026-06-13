//! Resolver-façade proofs. These run with NO daemon, so every `fetch` misses —
//! which is exactly the fail-closed contract under test: a miss MUST return the
//! caller's compiled default, never a fabricated value, never a panic.
use super::{gate_enabled, gate_patterns, gate_text, gate_threshold};

#[test]
fn threshold_falls_back_to_default_when_no_override() {
    // No daemon in the unit-test process -> miss -> compiled default.
    assert!((gate_threshold("kavach-rs", "dup.near", 0.85) - 0.85).abs() < f64::EPSILON);
}

#[test]
fn enabled_falls_back_to_default() {
    assert!(gate_enabled("kavach-rs", "some.gate", true));
    assert!(!gate_enabled("kavach-rs", "other.gate", false));
}

#[test]
fn text_falls_back_to_default() {
    assert_eq!(gate_text("kavach-rs", "contract", "DEFAULT"), "DEFAULT");
}

#[test]
fn empty_project_or_key_is_a_miss() {
    assert!((gate_threshold("", "k", 1.0) - 1.0).abs() < f64::EPSILON);
    assert!((gate_threshold("p", "", 2.0) - 2.0).abs() < f64::EPSILON);
}

#[test]
fn patterns_return_the_compiled_floor_when_no_override() {
    // The security-floor invariant: with no DB row, the result is EXACTLY the
    // compiled default — no patterns dropped, none added.
    let floor = ["rm -rf", "DROP TABLE"];
    let resolved = gate_patterns("kavach-rs", "bash.blocked", &floor);
    assert_eq!(resolved, vec!["rm -rf".to_owned(), "DROP TABLE".to_owned()]);
}
