//! `[PHASE]` block plus RIR (Recency-Importance-Relevance) context pruning that
//! scales injection depth to the session's `context_phase`.
use std::fmt::Write as _;

use kavach_session::SessionState;

/// Append the `[PHASE]` block and phase-aware context-pruning hints.
pub(super) fn append_phase_and_rir(context: &mut String, session: &mut SessionState) {
    let phase = if session.current_phase.is_empty() {
        "PLAN"
    } else {
        &session.current_phase
    };
    let iteration = if session.current_iteration_file.is_empty() {
        "(none)"
    } else {
        &session.current_iteration_file
    };
    let phase_done_count = session.iteration_files_done.len();
    writeln!(
        context,
        "\n[PHASE]\ncurrent: {phase}\niteration: {iteration}\nfiles_done: {phase_done_count}"
    )
    .ok();

    // No budget-driven throttle: caps removed (decision.remove-context-budget-caps).
    // The model works at full fidelity at every fill level; Claude Code auto-compact
    // reclaims context losslessly at the boundary. Always inject the normal modules.
    let module_ctx = session.inject_modules_once(&["agi-flow", "memory"]);
    context.push_str(&module_ctx);
}
