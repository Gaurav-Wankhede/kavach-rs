//! Layer-A bandit-log emit seam (harness-rl Wave P2b).
//!
//! Both instrumented gates (`PreToolUse`, Stop) call [`emit::emit_decision`] to log
//! one RLVR tuple `(context x, action a, propensity p, reward r)` to the daemon's
//! `bandit_log` store via `db.bandit_row`. Pure logging — NO gate behavior change,
//! fire-and-forget so a down daemon never blocks a gate.

pub(crate) mod emit;
