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

    // Claude Code's autonomous auto-compact reclaims context losslessly at the
    // window boundary — it preserves state, the model does not. So this gate must
    // NOT instruct the model to run /compact, nor tell it to "skip context" or
    // "be terse" (both DROP signal the auto-compact would have kept). At every
    // budget tier the model keeps working at full fidelity; auto-compact handles
    // reclamation. Emit a neutral, non-actionable budget telemetry line only.
    // SOURCE: <https://docs.claude.com/en/docs/claude-code/costs#auto-compact>
    match session.context_phase.as_str() {
        "critical" => {
            context.push_str("\n[CONTEXT_BUDGET] >90% used — auto-compact will reclaim at the boundary; continue normally.\n");
        }
        "late" => {
            context.push_str(
                "\n[CONTEXT_BUDGET] >70% used — auto-compact active; continue normally.\n",
            );
        }
        _ => {
            let module_ctx = session.inject_modules_once(&["agi-flow", "memory"]);
            context.push_str(&module_ctx);
        }
    }
}
