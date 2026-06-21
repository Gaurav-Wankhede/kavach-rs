//! Proves the loophole signal is RESOLVE-not-block: a loophole advisory on a
//! drained board is attached as a clean-exit ride-along and NEVER refuses the
//! stop. The refuse-stop block was removed (decision.loophole.resolve-not-handback).
use crate::gates::stop::shared::StopCtx;
use kavach_types::HookInput;

fn ctx_with<'a>(
    input: &'a HookInput,
    session: &'a mut kavach_session::SessionState,
    loophole: Option<String>,
) -> StopCtx<'a> {
    StopCtx {
        input,
        session,
        semver_advisory: None,
        capture_advisory: None,
        loophole_advisory: loophole,
        shallow_advisory: None,
        continuation_advisory: None,
        research_unsourced: false,
    }
}

#[test]
fn loophole_advisory_is_carried_not_blocked() {
    // The advisory is held on the ctx for clean_exit to append as a ride-along;
    // there is no refuse-stop path that could halt the loop on it.
    let input = HookInput::default();
    let mut session = kavach_session::SessionState::default();
    let ctx = ctx_with(&input, &mut session, Some("[LOOPHOLE_SURFACE] x".to_owned()));
    assert!(ctx.loophole_advisory.is_some(), "advisory is surfaced, never suppressed");
}

#[test]
fn absent_loophole_advisory_is_none() {
    let input = HookInput::default();
    let mut session = kavach_session::SessionState::default();
    let ctx = ctx_with(&input, &mut session, None);
    assert!(ctx.loophole_advisory.is_none());
}
