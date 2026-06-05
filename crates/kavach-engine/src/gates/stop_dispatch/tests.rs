//! `stop_dispatch` tests: saturation-breaker invariants + auto-verify outcome.
use super::card::is_backlog_saturated;
use super::verify::AutoVerify;

// Regression: the auto-verify outcome MUST keep three states distinct. A prior
// `usize` return collapsed `NothingDone` (empty/owner-gated → clean stop) and
// `WitnessFailed` (AI repair work) both to 0, trapping the stop gate in an
// infinite KEYSTONE_REPAIR loop on owner-gated backlogs. These must NOT be equal.
#[test]
fn auto_verify_states_are_distinct() {
    assert_ne!(AutoVerify::NothingDone, AutoVerify::WitnessFailed);
    assert_ne!(AutoVerify::NothingDone, AutoVerify::Promoted(0));
    assert_ne!(AutoVerify::WitnessFailed, AutoVerify::Promoted(0));
    // Only Promoted(n>0) drives re-dispatch; Promoted(0) is a clean-stop branch.
    assert_ne!(AutoVerify::Promoted(0), AutoVerify::Promoted(1));
}

// mirror SessionState::max_stop_reblocks()
const MAX_REBLOCK: i32 = 3;

#[test]
fn making_progress_never_saturates_however_high_the_breaker() {
    for n in [0, 1, MAX_REBLOCK, MAX_REBLOCK + 5, i32::MAX] {
        assert!(
            !is_backlog_saturated(n, true),
            "progress made -> must NOT abandon a non-empty queue (breaker={n})"
        );
    }
}

#[test]
fn within_breaker_ceiling_never_saturates_even_without_progress() {
    for n in [0, 1, 2, MAX_REBLOCK] {
        assert!(
            !is_backlog_saturated(n, false),
            "breaker={n} <= ceiling -> queue still authoritative, do not stop"
        );
    }
}

#[test]
fn stuck_card_past_ceiling_with_no_progress_finally_saturates() {
    assert!(
        is_backlog_saturated(MAX_REBLOCK + 1, false),
        "stuck card past ceiling with no progress must eventually yield"
    );
    assert!(is_backlog_saturated(i32::MAX, false));
}
