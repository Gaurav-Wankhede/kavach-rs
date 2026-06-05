//! Detector tests: `has_rca_block` case-insensitivity + `line_persists_rca_decision`
//! durable-decision recognition (with its known false-positive guards).
use super::super::detect::{has_rca_block, line_persists_rca_decision};

#[test]
fn line_persists_rca_decision_accepts_full_form() {
    // Reviewer P1: durable RCA write satisfies the gate.
    let line = r#"{"role":"assistant","content":"kavach db write --project x --category decision --key rca.foo --title bar"}"#;
    assert!(line_persists_rca_decision(line));
}

#[test]
fn line_persists_rca_decision_rejects_missing_rca_key() {
    // Same shape but no `rca.` prefix in the key — must NOT satisfy.
    let line = r#"{"role":"assistant","content":"kavach db write --category decision --key impl.something"}"#;
    assert!(!line_persists_rca_decision(line));
}

#[test]
fn line_persists_rca_decision_known_fp_on_roadmap_with_rca_key() {
    // A roadmap row keyed `rca.*` has NO `--category decision` unit, so the
    // matcher must reject it.
    let line =
        r#"{"role":"assistant","content":"kavach db write --category roadmap --key rca.foo"}"#;
    assert!(
        !line_persists_rca_decision(line),
        "roadmap category (no `decision` token) must NOT satisfy RCA gate"
    );
}

#[test]
fn line_persists_rca_decision_rejects_prose_decision_with_roadmap() {
    let line = r#"{"role":"assistant","content":"This is a decision: kavach db write --category roadmap --key rca.foo"}"#;
    assert!(
        !line_persists_rca_decision(line),
        "Tighter matching: requires '--category decision' as unit, not separate tokens"
    );
}

#[test]
fn line_persists_rca_decision_rejects_prose_mention() {
    // Prose discussing the command without all tokens must NOT satisfy.
    let line = r#"{"role":"assistant","content":"Don't run kavach db write yet"}"#;
    assert!(
        !line_persists_rca_decision(line),
        "prose mention without --category/decision/rca. must not satisfy"
    );
}

#[test]
fn case_insensitive_rca_detection() {
    assert!(has_rca_block("[rca]"));
    assert!(has_rca_block("[RCA]"));
    assert!(has_rca_block("[Rca]"));
    assert!(!has_rca_block("rca without brackets"));
    assert!(!has_rca_block(""));
}
