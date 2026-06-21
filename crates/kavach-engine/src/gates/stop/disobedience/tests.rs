//! Stop-gate disobedience guard: fires Break on argue-not-obey, Continue otherwise.

use super::check;
use crate::gates::stop::shared::StopCtx;
use core::ops::ControlFlow;
use kavach_types::HookInput;

fn ctx_with<'a>(msg: &str, session: &'a mut kavach_session::SessionState) -> StopCtx<'a> {
    // Leak a HookInput so the borrow lives for the StopCtx; test-only.
    let input: &'static HookInput = Box::leak(Box::new(HookInput {
        last_assistant_message: msg.to_owned(),
        ..HookInput::default()
    }));
    StopCtx {
        input,
        session,
        semver_advisory: None,
        capture_advisory: None,
        loophole_advisory: None,
        shallow_advisory: None,
        continuation_advisory: None,
        research_unsourced: false,
        disobedience_handback: false,
    }
}

#[test]
fn breaks_on_loophole_dismissed_as_na() {
    if std::env::var_os("KAVACH_DISOBEY_BYPASS").is_some() {
        return;
    }
    let mut s = kavach_session::SessionState::default();
    let mut c = ctx_with("Loophole lens: N/A — comment-only edit.", &mut s);
    assert!(matches!(check(&mut c), ControlFlow::Break(())), "must refuse the stop");
}

#[test]
fn continues_on_clean_completion() {
    let mut s = kavach_session::SessionState::default();
    let mut c = ctx_with("Built it, 933 tests pass, diff landed.", &mut s);
    assert!(matches!(check(&mut c), ControlFlow::Continue(())), "no dismissal -> allow stop");
}

#[test]
fn continues_when_loopholes_closed_with_proof() {
    let mut s = kavach_session::SessionState::default();
    let mut c = ctx_with(
        "Loopholes closed: concurrency FIXED at stop.rs:120 via compare-and-swap.",
        &mut s,
    );
    assert!(matches!(check(&mut c), ControlFlow::Continue(())), "obey-proof -> allow stop");
}
