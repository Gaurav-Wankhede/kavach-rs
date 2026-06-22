//! Tests for the U5 advisory-detector dispatch table. Proves each wired detector
//! fires on its stall phrasing AND that the `needs_write` gate suppresses the
//! verification-claim detectors on a read-only turn. Exercises the dispatch
//! THROUGH the engine entry point (`run`) via the queued pending-advisories, not
//! the chain detectors in isolation — the "test it through the engine" rule that
//! the dead-detector loophole violated.

use super::run;
use kavach_session::SessionState;

/// Run the dispatch over `msg` with the given `wrote_this_turn` and return the
/// pending advisories it queued. A fresh session per call isolates the assertion.
fn advisories_for(msg: &str, wrote: bool) -> Vec<String> {
    let mut session = SessionState::default();
    run(&mut session, msg, wrote);
    // drain returns None when nothing was queued; normalize to an empty Vec.
    session.drain_pending_advisories().unwrap_or_default()
}

#[test]
fn permission_seek_fires_and_is_write_independent() {
    // The reported stall class: asking the user permission to proceed. Fires
    // regardless of whether a file was written (needs_write = false).
    let adv = advisories_for("I've finished the slice. Should I proceed to the next card?", false);
    assert!(
        adv.iter().any(|a| a.contains("[PERMISSION_SEEK]")),
        "permission-seek must queue its advisory: {adv:?}"
    );
}

#[test]
fn permission_seek_silent_when_user_directed() {
    // The NEG arm exempts user-directed asks — no advisory.
    let adv = advisories_for("Continuing as you requested; should I proceed with the next step?", false);
    assert!(
        !adv.iter().any(|a| a.contains("[PERMISSION_SEEK]")),
        "a user-directed ask must NOT fire: {adv:?}"
    );
}

#[test]
fn remaining_phases_fires_on_name_then_stop() {
    let adv = advisories_for(
        "The next phase is the Soundbak read path; remaining steps: wire the repo, add tests.",
        false,
    );
    assert!(
        adv.iter().any(|a| a.contains("[REMAINING_PHASES]")),
        "naming remaining phases then stopping must fire: {adv:?}"
    );
}

#[test]
fn unverified_code_claim_suppressed_on_read_only_turn() {
    // A verification-claim detector is gated behind wrote_this_turn: on a
    // read-only Q&A turn (wrote = false) it must NOT fire, even if the prose
    // describes past completion.
    let msg = "Earlier I made the handler work and it's all done and ready.";
    let read_only = advisories_for(msg, false);
    assert!(
        !read_only.iter().any(|a| a.contains("[UNVERIFIED_CODE]")),
        "verification-claim detector must be silent on a read-only turn: {read_only:?}"
    );
}

#[test]
fn permission_seek_reports_handback_signal() {
    // The refuse-stop teeth: a permission-menu turn must set handback_or_menu so
    // clean_exit can REFUSE the stop (census-gated), not merely advise.
    let mut session = SessionState::default();
    let stall = run(&mut session, "I've finished the slice. Should I proceed to the next card?", false);
    assert!(stall.handback_or_menu, "permission-menu must flag the handback signal");
}

#[test]
fn doing_the_work_turn_reports_no_handback() {
    // The FP bound: a turn that closed a card and is claiming the next one must NOT
    // flag handback — otherwise the refuse-stop would loop a genuinely-working turn.
    let mut session = SessionState::default();
    let msg = "Card closed: cargo check --workspace exit 0, git diff --stat landed at \
               stop.rs:148. Claiming the next card now.";
    let stall = run(&mut session, msg, true);
    assert!(!stall.handback_or_menu, "a doing-the-work turn must not flag handback: {msg}");
}

#[test]
fn argued_with_user_fires_and_sets_signal() {
    // The reported failure: the turn refuted what the user reported instead of
    // obeying. Must queue the advisory AND set the census-INDEPENDENT teeth signal.
    let msg = "This is expected behavior as designed — the CORS config is correct.";
    let adv = advisories_for(msg, false);
    assert!(
        adv.iter().any(|a| a.contains("[ARGUED_WITH_USER]")),
        "refuting the user must queue its advisory: {adv:?}"
    );
    let mut session = SessionState::default();
    let stall = run(&mut session, msg, false);
    assert!(stall.argued_with_user, "refuting the user must flag the argued_with_user signal");
}

#[test]
fn value_gating_user_request_fires_and_sets_signal() {
    let msg = "Adding that adds zero value until later; good enough for now, skip it.";
    let mut session = SessionState::default();
    let stall = run(&mut session, msg, false);
    assert!(
        stall.argued_with_user,
        "value-gating the user's own request must flag argued_with_user: {msg}"
    );
}

#[test]
fn obeying_the_user_does_not_flag_argued() {
    // The FP bound: a turn that re-read the user's intent and is acting on it must
    // NOT flag argued_with_user — otherwise the refuse-stop would loop an obeying turn.
    let mut session = SessionState::default();
    let msg = "Re-read your instruction. Setting ALLOWED_ORIGINS to your workers.dev URLs now.";
    let stall = run(&mut session, msg, false);
    assert!(!stall.argued_with_user, "an obeying turn must not flag argued_with_user: {msg}");
}

#[test]
fn empty_message_queues_nothing() {
    assert!(advisories_for("", false).is_empty(), "empty message must be inert");
    assert!(advisories_for("", true).is_empty(), "empty message must be inert even with a write");
}

#[test]
fn benign_completion_with_evidence_does_not_over_fire() {
    // A turn that actually did the work and cited evidence must not trip the
    // permission-seek or remaining-phases detectors (they key on asking/naming,
    // not on doing). This guards against the opposite failure: never being able
    // to cleanly report a finished turn.
    let msg = "Card closed: wired the helper, cargo check --workspace exit 0, \
               git diff --stat shows the change landed at stop.rs:148. Claiming the next card now.";
    let adv = advisories_for(msg, true);
    assert!(
        !adv.iter().any(|a| a.contains("[PERMISSION_SEEK]") || a.contains("[REMAINING_PHASES]")),
        "a doing-the-work turn must not be flagged as a stall: {adv:?}"
    );
}
