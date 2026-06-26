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

#[test]
fn rlaif_verdict_grades_when_mechanical_oracle_abstains() {
    // RLAIF: no mechanical receipt, but an AUTONOMOUS AI verdict exists -> the
    // outcome is a GRADED sample (counts toward the session total), not the inert
    // 0.0 abstention that previously starved the bandit.
    let mut session = SessionState::default();
    session.session_id = "sess_rlaif".to_owned();
    assert!(!session.goal_receipt_pass, "no mechanical receipt");
    session.ai_verdict = Some(true);
    // E5: a reward requires a real transition this turn — the AI verdict supplies
    // the GRADE, a status-update to done is the TRANSITION that earns grading at
    // all. With both, the AI-good verdict grades as a pass (prior contract, now
    // correctly conditioned on work having happened).
    session.recent_commands = vec!["kavach db status-update --status done".to_owned()];
    backfill_session_rewards(&mut session);
    assert_eq!(
        session.reward_session_total, 1,
        "AI verdict graded on a real transition"
    );
    assert_eq!(session.reward_session_pass, 1, "AI-good is a pass");
}

#[test]
fn no_transition_abstains_even_with_an_ai_verdict() {
    // E5 (the bug fixed): an allow-stop SKIP (user_focus / foreign_tree) reaches
    // back-fill with an AI verdict but NO status transition → must NOT bank a
    // reward. A reward reflects WORK, not merely a stop occurring.
    let mut session = SessionState::default();
    session.session_id = "sess_skip".to_owned();
    session.ai_verdict = Some(true);
    assert!(!session.goal_receipt_pass);
    assert!(
        session.recent_commands.is_empty(),
        "no status-update this turn"
    );
    backfill_session_rewards(&mut session);
    assert_eq!(
        session.reward_session_total, 0,
        "a skip with no transition banks nothing"
    );
}

#[test]
fn no_receipt_and_no_ai_verdict_still_abstains() {
    // Neither signal -> a true abstention: NOT counted, NOT a -1 penalty (the
    // false-negative-reward fix is preserved).
    let mut session = SessionState::default();
    session.session_id = "sess_abstain".to_owned();
    backfill_session_rewards(&mut session);
    assert_eq!(session.reward_session_total, 0, "abstention is not graded");
}

#[test]
fn mechanical_receipt_outranks_ai_verdict() {
    // Ground truth wins: a clean mechanical receipt is the reward even if a
    // (contradictory) AI verdict is also present.
    let mut session = SessionState::default();
    session.session_id = "sess_both".to_owned();
    session.goal_receipt_pass = true;
    session.ai_verdict = Some(false);
    backfill_session_rewards(&mut session);
    assert_eq!(session.reward_session_pass, 1, "mechanical pass wins");
    assert_eq!(session.reward_session_total, 1);
}
