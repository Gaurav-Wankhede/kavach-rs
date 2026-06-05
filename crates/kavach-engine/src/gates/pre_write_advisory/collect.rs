//! Stage 4 orchestrator: chain every section-8 non-blocking advisory into one
//! approval-context string. Each group lives in a sibling appender module.
use kavach_types::HookInput;

use super::append::push_opt;
use super::counterfactual::counterfactual_advisory;
use super::guards::{append_craft_guards, append_lang_guards, append_platform_guards};
use super::memory::memory_awareness_advisory;
use crate::gates::pre_write_checks::build_approval_context;
use crate::gates::pre_write_context::WriteContext;

/// Collect all advisory context for the approval response.
pub(crate) fn collect(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &mut kavach_session::SessionState,
    runner_compact: &str,
    algo_advisory: Option<&str>,
) -> String {
    let mut context = build_approval_context(ctx.tool_name, runner_compact, session);

    push_opt(
        &mut context,
        super::super::new_package_guard::check_new_package(ctx.file_path),
    );
    push_opt(
        &mut context,
        non_empty(super::super::rag_router::advisory_context_all(
            ctx.file_path,
            ctx.content,
            &session.intent_type,
            3,
        )),
    );
    push_opt(
        &mut context,
        super::super::pre_write_checks::detect_bulk_checkbox(
            ctx.file_path,
            input.get_string("content"),
            input.get_string("new_string"),
        ),
    );

    append_lang_guards(ctx, &mut context);
    append_platform_guards(ctx, &mut context);

    // Karpathy Principle 1: Think First (stateful — records the injection).
    if let Some(tf) = super::super::elicitation::think_first_advisory(
        &session.intent_type,
        &session.intent_risk,
        session.think_first_injected,
    ) {
        push_opt(&mut context, Some(tf));
        session.think_first_injected = true;
        session.save().ok();
    }

    push_opt(&mut context, counterfactual_advisory(ctx.content));
    append_craft_guards(ctx, input, &session.files_modified_this_turn, &mut context);
    push_opt(&mut context, algo_advisory.map(ToOwned::to_owned));

    if !session.project.is_empty() {
        push_opt(&mut context, memory_awareness_advisory(&session.project));
    }

    context
}

/// `None` for an empty advisory string so `push_opt` skips the separator.
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}
