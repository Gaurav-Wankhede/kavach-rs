//! Tests for the disk-pressure self-heal directive text.
//!
//! `is_critically_low` / `maybe_self_heal` read the real DB volume's free space,
//! which is environment-dependent and not deterministically assertable in a unit
//! test. The behavior under test here is the DIRECTIVE: it must be ACT-driven and
//! must NOT contain the abolished operator-handback patterns the transcript
//! surfaced. The threshold-probe seam is covered by the integration build.

use super::self_heal_directive;

#[test]
fn directive_is_act_driven_not_handback() {
    let d = self_heal_directive(130);
    // It reports the measured free space …
    assert!(d.contains("130 MiB free"), "directive must state measured headroom");
    // … and orders the agent to free space ITSELF.
    assert!(d.contains("YOU free the space"), "must be agent-self-heal");
    assert!(d.contains("cargo clean"), "must name a concrete reclaim action");
    assert!(d.contains("DISK_RECLAIM"), "must carry the ACT marker tag");
}

#[test]
fn directive_forbids_operator_handback_phrases() {
    let d = self_heal_directive(130);
    // The exact surrender phrases from the motivating transcript must be named as
    // FORBIDDEN, never emitted as an instruction TO the operator.
    assert!(
        d.contains("FORBIDDEN"),
        "directive must explicitly forbid the handback patterns"
    );
    assert!(
        d.contains("Owner — run"),
        "directive must name the 'Owner — run' anti-pattern to forbid it"
    );
    assert!(
        d.contains("Holding"),
        "directive must name the 'Holding' anti-pattern to forbid it"
    );
    // And it must NOT instruct an `rm -rf ~/.cache/...` for the operator to run.
    assert!(
        !d.contains("operator run") && !d.contains("run in your terminal"),
        "directive must never hand a command to the operator"
    );
}
