//! Guard: the genuine clean exit. No pending tasks, no review needed — reset
//! the pending-work re-block breaker and emit either a STOP context (when a
//! reason or semver advisory exists) or a silent exit. Always Breaks (terminal).

use core::ops::ControlFlow;

use super::super::shared::StopCtx;
use crate::gates::bandit::emit;
use kavach_patterns::bandit_log::{BanditContext, GateAction};

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // Layer-A bandit log: a clean exit is the Stop gate's `Allow` action. Reward
    // is None here — it is back-filled when the 3-witness verify resolves. Pure
    // logging, fire-and-forget; never affects whether the stop proceeds.
    emit::emit_decision(
        &ctx.session.session_id,
        BanditContext::new(
            "stop",
            "Stop",
            "",
            0,
            "",
            u32::try_from(ctx.session.turn_count).unwrap_or(0),
        ),
        GateAction::Allow,
        1.0,
        None,
    );
    let semver_ctx = ctx.semver_advisory.as_deref().unwrap_or("");
    // U3 capture-finding nudge: non-blocking — it NEVER prevents the clean stop,
    // it only rides along in the STOP context when a decision was settled in
    // prose but not persisted this turn.
    // The capture nudge is RECORDED to the mistake ledger in stop.rs (at the
    // computation site, so it fires on every stop, not just this terminal one);
    // here it only rides along in the clean-exit STOP context when this branch
    // is actually reached.
    let capture_ctx = ctx.capture_advisory.as_deref().unwrap_or("");
    // Work is genuinely done — reset the pending-work re-block breaker.
    ctx.session.clear_stop_reblock();
    if !ctx.input.reason.is_empty() || !semver_ctx.is_empty() || !capture_ctx.is_empty() {
        let context = kavach_hook::context_block(
            "STOP",
            &[
                (
                    "why",
                    if ctx.input.reason.is_empty() {
                        "clean"
                    } else {
                        &ctx.input.reason
                    },
                ),
                ("t", &ctx.session.turn_count.to_string()),
            ],
        );
        // Append the capture nudge after the structured block; it is advisory
        // text, so the stop still proceeds — the agent sees it next turn.
        let full = if capture_ctx.is_empty() {
            context
        } else {
            format!("{context}\n{capture_ctx}")
        };
        drop(kavach_hook::exit_stop_context(&full));
    } else {
        drop(kavach_hook::exit_silent());
    }
    ControlFlow::Break(())
}
