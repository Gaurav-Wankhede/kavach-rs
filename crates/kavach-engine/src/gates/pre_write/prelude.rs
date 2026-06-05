//! Stage 0 pre-checks: SDLC-phase + iteration-scope advisories. Both are
//! demoted-to-advisory nudges (per roadmap.unit.gate-severity-classification)
//! that short-circuit the pipeline with an allow when they fire.
use crate::gates::pre_write_context::WriteContext;

/// Return an advisory string if a Stage 0 pre-check fires (PLAN-phase code write
/// or cross-iteration switch), else `None` to continue the pipeline.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    phase_advisory(ctx, session).or_else(|| iteration_advisory(ctx, session))
}

/// SDLC Phase enforcement — PLAN phase nudges away from code writes.
/// ARCH: `PhaseGateEnforcement`. Per ~/.claude/rules/10-reinforce.md §SDLC Phase Gates.
fn phase_advisory(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    let phase = if session.current_phase.is_empty() {
        "PLAN"
    } else {
        session.current_phase.as_str()
    };
    (phase == "PLAN" && ctx.is_code && !ctx.is_test).then(|| {
        "[ADVISORY] PHASE: in PLAN phase; consider `kavach phase advance` if implementing"
            .to_owned()
    })
}

/// Iteration scope — one file at a time, full depth. Compares canonicalized
/// paths so relative vs absolute spellings of the same file resolve identically.
fn iteration_advisory(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    if session.current_iteration_file.is_empty() || ctx.file_path.is_empty() {
        return None;
    }
    let canonical_ctx = kavach_session::canonicalize_iteration_path(ctx.file_path);
    if canonical_ctx == session.current_iteration_file
        || ctx.file_path == session.current_iteration_file
    {
        return None;
    }
    let current = &session.current_iteration_file;
    Some(format!(
        "[ADVISORY] ITERATION_SCOPE: current iteration is {current}; switching to {}",
        ctx.file_path
    ))
}
