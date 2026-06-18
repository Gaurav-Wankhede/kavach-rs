//! Proves the loophole refuse-stop decision (parity with the cycle deadlock):
//! an un-closed loophole on a drained board REFUSES the clean stop, bounded by
//! the behavioral breaker so it can never spin forever.
use super::refuse_stop_on_open_loophole;
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
    }
}

#[test]
fn no_loophole_advisory_never_refuses_stop() {
    let input = HookInput::default();
    let mut session = kavach_session::SessionState::default();
    let mut ctx = ctx_with(&input, &mut session, None);
    assert!(
        !refuse_stop_on_open_loophole(&mut ctx),
        "a clean turn (no open loophole) stops cleanly"
    );
}

#[test]
fn open_loophole_refuses_stop_then_breaker_force_allows() {
    let input = HookInput::default();
    let mut session = kavach_session::SessionState::default();
    // Default circuit-breaker threshold is 3: the 3rd consecutive block trips,
    // after which the gate force-allows (loop-safety) while recording the
    // surrender — so the refusal is bounded, never an infinite spin.
    session.gate_circuit_breaker_threshold = 3;

    let mut refusals = 0;
    for _ in 0..6 {
        let mut ctx = ctx_with(&input, &mut session, Some("[LOOPHOLE] open".to_owned()));
        if refuse_stop_on_open_loophole(&mut ctx) {
            refusals += 1;
        }
    }
    // It refuses for the first (threshold - 1) calls, then the breaker trips and
    // every subsequent call force-allows — so the count is bounded, NOT 6.
    assert_eq!(refusals, 2, "refuses up to the breaker bound, then force-allows");
    assert!(
        session.is_gate_tripped("loophole_open"),
        "the breaker tripped, guaranteeing loop-safety"
    );
}
