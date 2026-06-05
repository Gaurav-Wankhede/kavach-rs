//! `SessionStart` gate: detect model, reset stale state, inject boot context.
//!
//! Submodules: `state` (model/reset/title), `boot` (skill registry), `rag`
//! (tree refresh), `memory` (project/ancestry titles), `patterns` (hot
//! patterns + mistake ledger), `context` (assemble the `[SESSION_START]` block).
mod boot;
mod context;
mod memory;
mod patterns;
mod rag;
mod state;

#[cfg(test)]
mod tests;

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
    // Session-aware load: resolve the durable session_runtime DB row for THIS
    // session_id. A /clear starts a new session_id, so this returns fresh
    // state instead of rehydrating the prior conversation's INI file — the
    // root cause of cross-session state drift (stale files_modified /
    // research_done). SessionStart owns the session boundary; later gates'
    // plain get_or_create_session() read back the corrected INI state.
    let mut session = kavach_session::get_or_create_session_for(&input.session_id);

    state::set_model(&mut session, input);
    state::reset_stale_state(&mut session);

    boot::build_skill_registry();
    rag::refresh_all_rag_trees();
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
