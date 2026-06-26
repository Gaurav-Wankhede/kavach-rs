//! P3a reward back-fill trigger (harness-rl) — the stop-time half of the JOIN.
//!
//! When the Stop gate closes a session it knows two things the Layer-A emit did
//! not: the `session_id` AND the session's 3-witness verify outcome (`n_pass`).
//! Those are exactly the join key + the reward signal the `bandit_log` rows are
//! waiting on, so here we fire `db.bandit_backfill_session` to grade that
//! session's un-rewarded decisions. Fire-and-forget: a down daemon must never
//! block or alter the Stop hook (which carries security duties).

use kavach_session::{RewardOutcome, SessionState};

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
    // Reward only on real work completion: 3-witness receipt or status-update to done/verified.
    // See decision.engine.reward-gate-no-spurious-skip.
    let transitioned = session.goal_receipt_pass
        || session.recent_commands.iter().any(|c| {
            c.contains("status-update")
                && (c.contains("--status done") || c.contains("--status verified"))
        });
    if !transitioned {
        return;
    }
    let card = if session.current_kanban_card.is_empty() {
        "session".to_owned()
    } else {
        session.current_kanban_card.clone()
    };
    // Reward resolution: (1) 3-witness receipt, (2) AI verdict, (3) project rubric.
    // See decision.engine.reward-resolution-rlaif.
    let outcome = if session.goal_receipt_pass {
        RewardOutcome::Passed
    } else if let Some(v) = session.ai_verdict {
        RewardOutcome::AiJudged(v)
    } else {
        rubric_outcome(session)
    };
    session.record_reward_outcome(&card, outcome);
    // Abstention writes no reward — a missing signal must never become a -1
    // penalty on the bandit log. Only a GRADED outcome (mechanical OR AI-judged)
    // fires the grader, carrying its ±1 sign as the `verified_clean` signal.
    let Some(verified_clean) = outcome.graded() else {
        return;
    };
    let params = serde_json::json!({
        "session_id": session.session_id,
        "verified_clean": verified_clean,
        "limit": BACKFILL_LIMIT,
    });
    // Fire-and-forget but NON-LOSSY: daemon-down is spooled + replayed next Stop,
    // so a reward signal is never lost; the Stop gate still never blocks.
    super::spool_writes::call_or_spool("db.bandit_backfill_session", &params);
}

/// Score the session's trajectory under its PROJECT-ADAPTIVE rubric and map the
/// scalar to a reward outcome: positive (verified work outweighs penalties) →
/// clean, negative (gate-block / deferral-handoff dominates) → not-clean, zero
/// (no signal either way) → abstain. The rubric is loaded per-project so a
/// non-Rust stack scores its own verify commands. Any read error → abstain
/// (a missing tape must never fabricate a reward). Operator directive 2026-06-17.
fn rubric_outcome(session: &SessionState) -> RewardOutcome {
    let Ok(path) = kavach_patterns::eval_replay::default_trajectory_path(&session.session_id)
    else {
        return RewardOutcome::Abstain;
    };
    let Ok(events) = kavach_patterns::eval_replay::read_jsonl(&path) else {
        return RewardOutcome::Abstain;
    };
    let rubric = crate::gates::stop_dispatch::reward_rubric_for(&session.project);
    // DB-sourced multidimensional-oracle config (weights/penalty/failure-vocab as
    // DATA, not source literals); absent/malformed row → compiled default.
    let oracle_cfg = crate::gates::stop_dispatch::oracle_config_for(&session.project);
    match kavach_patterns::reward::score_trajectory_full(&events, &rubric, &oracle_cfg) {
        s if s > 0 => RewardOutcome::AiJudged(true),
        s if s < 0 => RewardOutcome::AiJudged(false),
        _ => RewardOutcome::Abstain,
    }
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
