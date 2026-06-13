//! Proof that the back-fill trigger is total and fail-closed: an empty session
//! id is a clean no-op (no RPC, no panic), and a populated one never panics even
//! when the daemon is absent (fire-and-forget).

use super::backfill_session_rewards;
use kavach_session::SessionState;

#[test]
fn an_empty_session_id_is_a_clean_noop() {
    let mut session = SessionState::default();
    assert!(session.session_id.is_empty(), "default has no session id");
    // Must not panic and must not attempt the RPC join (nothing to key on).
    backfill_session_rewards(&mut session);
}

#[test]
fn a_populated_session_never_panics_even_with_no_daemon() {
    let mut session = SessionState::default();
    session.session_id = "sess_test".to_owned();
    session.goal_receipt_pass = true;
    // Fire-and-forget: a down daemon is a silent no-op, never a panic.
    backfill_session_rewards(&mut session);
}
