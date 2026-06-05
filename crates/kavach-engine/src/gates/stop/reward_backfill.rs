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
pub(super) fn backfill_session_rewards(session: &SessionState) {
    if session.session_id.is_empty() {
        return;
    }
    let params = serde_json::json!({
        "session_id": session.session_id,
        "verified_clean": session.goal_receipt_pass,
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
