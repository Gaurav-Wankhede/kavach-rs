//! P3a reward back-fill trigger (harness-rl) — the stop-time half of the JOIN.
//!
//! When the Stop gate closes a session it knows two things the Layer-A emit did
//! not: the `session_id` AND the session's 3-witness verify outcome (`n_pass`).
//! Those are exactly the join key + the reward signal the `bandit_log` rows are
//! waiting on, so here we fire `db.bandit_backfill_session` to grade that
//! session's un-rewarded decisions. Fire-and-forget: a down daemon must never
//! block or alter the Stop hook (which carries security duties).

use kavach_session::SessionState;

#[cfg(test)]
#[path = "reward_backfill_test.rs"]
mod tests;

/// Max rows graded per stop — a generous cap on one session's decisions, so a
/// runaway log can never make the back-fill RPC unbounded.
const BACKFILL_LIMIT: u32 = 512;

/// Grade this session's logged bandit decisions against its verify outcome.
///
/// No-op on an empty `session_id` (nothing to join on). The reward signal is
/// `goal_receipt_pass` — true ONLY when a verified oracle receipt landed (a real
/// 3-witness), never a self-asserted "done". A hallucinated success cannot grade
/// a decision as clean. A passing session rewards its decisions as verified-clean;
/// a failing one penalizes only a false allow (the RPC's grading map decides per
/// action). Fire-and-forget.
pub(super) fn backfill_session_rewards(session: &mut SessionState) {
    if session.session_id.is_empty() {
        return;
    }
    let card = if session.current_kanban_card.is_empty() {
        "session".to_owned()
    } else {
        session.current_kanban_card.clone()
    };
    let verified = session.goal_receipt_pass;
    session.record_reward_outcome(&card, verified);
    let params = serde_json::json!({
        "session_id": session.session_id,
        "verified_clean": verified,
        "limit": BACKFILL_LIMIT,
    });
    // INTENTIONAL: fire-and-forget — daemon may be down; the Stop gate must not
    // block on reward bookkeeping.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> =
        kavach_rpc::client::call("db.bandit_backfill_session", Some(params));
}

/// z-score for ~95% pessimism in the stop-time learning pass.
const POLICY_Z: f64 = 1.96;
/// `DataCOPE` coverage floor (ESS ≥ 10% of n) the candidate must clear (GATE-A).
const POLICY_MIN_COVERAGE: f64 = 0.1;
/// Soft-vs-hard reward-hacking slack forwarded to the audit (GATE-B).
const POLICY_DRIFT_TOLERANCE: f64 = 0.05;

/// Learn from the freshly-graded rewards: fire `db.policy_improve` so the daemon
/// re-derives and — ONLY if trust coverage, the reward-hacking audit, and a
/// strict LCB win all clear — promotes a learned advisory policy into the graph.
///
/// Fire-and-forget, AFTER [`backfill_session_rewards`] (the rows it reads are
/// only graded once back-fill has run this stop). The promotion is fail-closed in
/// the daemon; here we just kick the pass. No-op on an empty `session_id`.
pub(super) fn trigger_policy_improve(session: &SessionState) {
    if session.session_id.is_empty() {
        return;
    }
    let params = serde_json::json!({
        "limit": BACKFILL_LIMIT,
        "z": POLICY_Z,
        "min_coverage_ratio": POLICY_MIN_COVERAGE,
        "drift_tolerance": POLICY_DRIFT_TOLERANCE,
    });
    // INTENTIONAL: fire-and-forget — daemon may be down; the Stop gate must not
    // block on policy learning.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> =
        kavach_rpc::client::call("db.policy_improve", Some(params));
}
