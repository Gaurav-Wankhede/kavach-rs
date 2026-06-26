//! Construct + fire one bandit-log row. Fire-and-forget RPC to `db.bandit_row`.
//!
//! The seam both instrumented gates share, so the context-build + serialize +
//! emit logic lives once and is tested once. A deterministic gate logs
//! propensity 1.0; the timestamp is read here (callers stay clock-free/testable
//! via [`build_row`]).

use kavach_patterns::bandit_log::{BanditContext, BanditRow, GateAction, Reward};

/// Build a row from its parts (clock-free — caller supplies `timestamp_ms`).
///
/// Kept separate from the RPC fire so tests assert the exact serialized shape
/// without a daemon. `reward` is `None` at decision time and back-filled later.
pub(crate) fn build_row(
    session_id: &str,
    timestamp_ms: i64,
    context: BanditContext,
    action: GateAction,
    propensity: f32,
    reward: Option<Reward>,
) -> BanditRow {
    let mut row = BanditRow::new(session_id, timestamp_ms, context, action, propensity);
    row.reward = reward;
    row
}

/// Serialize a row to the JSON payload the `db.bandit_row` RPC expects.
///
/// Returns `None` if serialization fails (it cannot for this type, but the emit
/// path stays fail-closed rather than unwrapping).
pub(crate) fn payload_of(row: &BanditRow) -> Option<String> {
    serde_json::to_string(row).ok()
}

/// Emit one decision row to the daemon (fire-and-forget).
///
/// No-op on an empty `session_id` (nothing to key the row to) or a serialize
/// failure. The RPC is fire-and-forget: a down daemon must never block the gate
/// that called us, so the result is intentionally discarded.
pub(crate) fn emit_decision(
    session_id: &str,
    context: BanditContext,
    action: GateAction,
    propensity: f32,
    reward: Option<Reward>,
) {
    if session_id.is_empty() {
        return;
    }
    let timestamp_ms = now_ms();
    let row = build_row(
        session_id,
        timestamp_ms,
        context,
        action,
        propensity,
        reward,
    );
    let Some(payload) = payload_of(&row) else {
        return;
    };
    let params = serde_json::json!({ "payload": payload });
    // INTENTIONAL: fire-and-forget — daemon may be down; gate must not block.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> = kavach_rpc::client::call("db.bandit_row", Some(params));
}

/// Emit a SOFT held-out reward row for this decision (P8), IF the held-out
/// sampler fires for the given `roll` (a uniform draw in `[0,1)`).
///
/// The held-out channel is the reward-hacking audit's independent signal: a
/// sampled slice that will be re-verified by the SAME 3-witness back-fill but is
/// tagged `held_out: true` so `db.ope_audit` never mixes it into the on-policy
/// training estimate. Sampling rate is `KAVACH_RL_HELDOUT_RATE` (default 0.0 ⇒
/// disabled, the exact pre-P8 behavior). The row records the SAME action +
/// propensity the on-policy row did — only the channel differs. Fire-and-forget;
/// a no-op on an empty `session_id`, a roll above the rate, or a serialize fail.
pub(crate) fn maybe_emit_held_out(
    session_id: &str,
    context: BanditContext,
    action: GateAction,
    propensity: f32,
    roll: f32,
) {
    if session_id.is_empty() || roll >= held_out_rate() {
        return;
    }
    let row = build_row(session_id, now_ms(), context, action, propensity, None).into_held_out();
    let Some(payload) = payload_of(&row) else {
        return;
    };
    let params = serde_json::json!({ "payload": payload });
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> = kavach_rpc::client::call("db.bandit_row", Some(params));
}

/// The held-out sampling rate from `KAVACH_RL_HELDOUT_RATE`, clamped to `[0,1]`.
/// Absent / unparseable / non-finite ⇒ 0.0 (held-out channel disabled), the
/// fail-safe default: no held-out rows means the audit reads the soft channel as
/// non-informative ⇒ `Inconclusive` ⇒ not promotable (never a false-clear).
fn held_out_rate() -> f32 {
    std::env::var("KAVACH_RL_HELDOUT_RATE")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|r| r.is_finite())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
}

/// Wall-clock ms since the epoch; 0 if the clock is before the epoch.
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[path = "emit_test.rs"]
mod tests;
