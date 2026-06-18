//! Stage 4 context assembly: base advisory collection + P1 advisories from the
//! tiered guard system + the soft §LSP-FIRST advisory.
use kavach_types::HookInput;

use crate::gates::pre_write_context::WriteContext;
use crate::gates::pre_write_guards::GuardResult;

/// Build the allow-time additionalContext block from advisory sources.
pub(super) fn build(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &mut kavach_session::SessionState,
    guard_result: &GuardResult,
) -> String {
    let mut context = super::super::pre_write_advisory::collect(
        ctx,
        input,
        session,
        &guard_result.runner_compact,
        guard_result.algo_advisory.as_deref(),
    );

    if !guard_result.p1_advisories.is_empty() {
        context.push_str("\n\n[P1_ADVISORIES]\n");
        for advisory in &guard_result.p1_advisories {
            context.push_str(advisory);
            context.push('\n');
        }
    }

    // §LSP-FIRST advisory (P1, soft) — surfaces when the edit target is an
    // LSP-supported file type but no LSP diagnostic call has been recorded
    // this session. SOURCE: ~/.claude/CLAUDE.md §LSP-FIRST + gates/CLAUDE.md.
    if let Some(adv) = super::super::pre_write_lsp_first::advisory(ctx.file_path, session) {
        context.push_str("\n\n");
        context.push_str(&adv);
        context.push('\n');
    }

    if let Some(adv) = super::skill_match::advisory(ctx, &session.intent_type) {
        context.push_str("\n\n");
        context.push_str(&adv);
        context.push('\n');
    }

    // Compact `[LOOP]` on production code writes (loop-eng Phase 2).
    if ctx.is_code && !ctx.is_test {
        let loop_line = super::super::loop_frame::build_loop_compact(session, None);
        context.push_str("\n\n");
        context.push_str(&loop_line);
        context.push('\n');
    }

    context
}
