use kavach_types::HookInput;

use crate::error::EngineError;

/// `PostCompact` gate: inject compact summary for context recovery.
#[expect(
    clippy::unnecessary_wraps,
    reason = "callers ignore result; fn signature is part of gate contract"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let mut session = kavach_session::get_or_create_session();
    session.mark_post_compact();

    let summary = &input.compact_summary;
    let trigger = &input.trigger;

    let context = kavach_hook::context_block(
        "POST_COMPACT",
        &[("trigger", if trigger.is_empty() { "auto" } else { trigger })],
    );

    let mut full_context = context;

    // Inject case facts FIRST — these are critical and must NOT be summarized.
    if !session.case_facts.is_empty() {
        full_context.push_str("\n[CASE_FACTS — DO NOT SUMMARIZE]\n");
        for fact in &session.case_facts {
            full_context.push_str("- ");
            full_context.push_str(fact);
            full_context.push('\n');
        }
    }

    // LOSSLESS working-set reconstruction: rebuild the exact durable spine (active
    // card + TOUCHES + recent decisions) live from the DB and inject it BEFORE the
    // lossy summary, so the summary is a supplement and the spine is re-derived from
    // the store — not a summary-of-a-summary. Fail-soft: None off-daemon / empty
    // project → omitted. See decision.engine.lossless-working-set-reconstruction.
    if let Some(ws) = super::working_set::reconstruct(&session.project) {
        full_context.push_str(&ws);
    }

    if !summary.is_empty() {
        full_context.push_str("\n[COMPACT_SUMMARY]\n");
        let preview: String = summary.chars().take(500).collect();
        full_context.push_str(&preview);
        full_context.push('\n');
    }

    let module_ctx = session.inject_modules_once(&["agi-flow", "memory"]);
    full_context.push_str(&module_ctx);

    drop(kavach_hook::exit_post_tool_context(&full_context));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_compact_default() {
        let input = HookInput::default();
        run(&input).expect("post_compact run should not fail");
    }

    #[test]
    fn test_post_compact_with_summary() {
        let input = HookInput {
            compact_summary: "session was compacted".into(),
            trigger: "auto".into(),
            ..Default::default()
        };
        run(&input).expect("post_compact run with summary should not fail");
    }
}
