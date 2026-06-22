//! `SessionStart` gate: detect model, reset stale state, inject boot context.
//!
//! Submodules: `state` (model/reset/title), `boot` (skill registry), `rag`
//! (tree refresh), `memory` (project/ancestry titles), `patterns` (hot
//! patterns + mistake ledger), `context` (assemble the `[SESSION_START]` block).
mod boot;
mod concepts;
mod context;
mod flows;
mod gui;
mod memory;
mod patterns;
pub(in crate::gates) mod reconcile;
mod stack_fit;
mod state;

#[cfg(test)]
mod tests;

// Shared with the Stop gate: an auto-compact can fire a Stop before the next
// SessionStart reconciles the seam, so the Stop terminal also checks it.
pub(in crate::gates) use reconcile::reconcile_context;

use kavach_types::HookInput;

use crate::error::EngineError;

/// `SessionStart` gate: detect model ID, set token budget, init session.
/// Claude Code sends `model` field in `SessionStart` hook input.
///
/// # Errors
/// Returns `Ok(())` on every path; the `Result` matches the `run_gate`
/// dispatch contract so all gate handlers share one return type.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature is fixed by the run_gate dispatch table: every gate handler returns Result<(), EngineError>"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    // Load session row from DB — fresh state after /clear (session boundary).
    // See decision.engine.session-start-aware-load.
    let mut session = kavach_session::get_or_create_session_for(&input.session_id);

    state::set_model(&mut session, input);
    state::reset_stale_state(&mut session);

    // Heavy boot work (skill-dir scan + GUI bringup) is skipped under
    // `cargo test`/`nextest` and when KAVACH_SKIP_HEAVY_BOOT=1: rebuilding the
    // registry on every invocation would blow the per-test timeout under the
    // parallel workspace run (and is wasteful in CI). The gate's own logic still
    // runs; only the expensive registry refresh is bypassed.
    if !cfg!(test) && std::env::var("KAVACH_SKIP_HEAVY_BOOT").as_deref() != Ok("1") {
        boot::build_skill_registry();
        // Bring up the GUI (:7777) + SurrealDB at session start, autonomously —
        // idempotent + detached, so the dashboard is live without a manual command.
        gui::ensure_gui_up();
    }
    let context = context::build(&mut session);

    super::event_log::log_session(
        &session.session_id,
        "session_start",
        &session.model_id,
        &session.project,
    );

    drop(kavach_hook::exit_session_start_full(
        &context,
        true,
        &state::session_title(&session),
    ));
    Ok(())
}
