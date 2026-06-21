//! `stop_dispatch` tests: auto-verify outcome distinctness.
use super::verify::AutoVerify;

// Regression: the auto-verify outcome MUST keep four states distinct. A prior
// `usize` return collapsed `NothingDone` (empty → clean stop) and
// `WitnessFailed` (AI repair work) both to 0, trapping the stop gate in an
// infinite KEYSTONE_REPAIR loop. These must NOT be equal.
// Also: `Unprovable` (non-Rust no KAVACH_VERIFY_CMD set) requires explicit
// user action distinct from witness failure.
#[test]
fn auto_verify_states_are_distinct() {
    assert_ne!(AutoVerify::NothingDone, AutoVerify::WitnessFailed);
    assert_ne!(AutoVerify::NothingDone, AutoVerify::Promoted(0));
    assert_ne!(AutoVerify::WitnessFailed, AutoVerify::Promoted(0));
    assert_ne!(AutoVerify::WitnessFailed, AutoVerify::Unprovable);
    // Only Promoted(n>0) drives re-dispatch; Promoted(0) is a clean-stop branch.
    assert_ne!(AutoVerify::Promoted(0), AutoVerify::Promoted(1));
    // VerifyRpcDown (work proven, DB write failed) must NOT collapse into
    // Promoted(0)/NothingDone — that collapse re-dispatched a finished card on a
    // transient daemon outage instead of surfacing the outage loudly.
    assert_ne!(AutoVerify::VerifyRpcDown, AutoVerify::Promoted(0));
    assert_ne!(AutoVerify::VerifyRpcDown, AutoVerify::NothingDone);
    assert_ne!(AutoVerify::VerifyRpcDown, AutoVerify::WitnessFailed);
}
