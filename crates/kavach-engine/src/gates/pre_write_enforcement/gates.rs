//! Stages 2b–4b: memory-query, skill (ANY-OF + file-pattern), research, and
//! evidence-chain enforcement. Each returns `Some(reason)` to hard-block.
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;

/// Stage 2b: implement/debug/refactor at medium+ risk must query kavach-db first.
pub(super) fn memory_check(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    let is_low_risk = session.intent_risk == "low";
    let requires = matches!(
        session.intent_type.as_str(),
        "implement" | "debug" | "refactor"
    );
    if !is_low_risk && ctx.is_code && requires && !session.memory_queried {
        return Some(
            "MEMORY_QUERY_REQUIRED: Run `kavach db kanban --project <slug>` or \
             `kavach pipeline status --project <slug>` before writing code. \
             Chat history is NOT authoritative — it may be truncated or wrong. \
             The kavach-db is the single source of truth for project state."
                .to_owned(),
        );
    }
    None
}

/// Stage 3+3b: ANY-OF required-skill enforcement plus file-pattern skills.
pub(super) fn skill_check(
    ctx: &WriteContext<'_>,
    session: &mut kavach_session::SessionState,
) -> Option<String> {
    let is_low_risk = session.intent_risk == "low";
    if ctx.is_code && !session.required_skills.is_empty() && !is_low_risk {
        let any_invoked = session
            .required_skills
            .iter()
            .any(|s| session.invoked_skills.contains(s));
        if any_invoked {
            let invoked: Vec<&str> = session
                .required_skills
                .iter()
                .filter(|s| session.invoked_skills.contains(*s))
                .map(String::as_str)
                .collect();
            session.add_case_fact(&format!(
                "skill gate satisfied by: [{}]",
                invoked.join(", ")
            ));
        } else {
            let skills_dir = kavach_config::paths::skills_dir();
            let missing: Vec<&str> = session
                .missing_skills()
                .into_iter()
                .filter(|s| skills_dir.join(s).join("SKILL.md").exists())
                .collect();
            if !missing.is_empty() {
                return Some(format!(
                    "SKILL VIOLATION: Invoke at least one required skill before writing code: [{}]",
                    missing.join(", ")
                ));
            }
        }
    }
    if super::patterns::should_check_patterns(ctx, is_low_risk) {
        return super::super::pre_write_patterns::check_file_pattern_skills(ctx.file_path, session);
    }
    None
}

/// Stage 4+4b: research-before-code (skipped on `low` effort) + evidence-chain.
pub(super) fn research_check(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &kavach_session::SessionState,
) -> Option<String> {
    let is_subagent = !input.session_id.is_empty()
        && !session.session_id.is_empty()
        && input.session_id != session.session_id;
    let low_effort = input.effort_level().eq_ignore_ascii_case("low");
    if !is_subagent
        && !low_effort
        && let Some(reason) = super::super::research_guard::check(
            &session.intent_type,
            input.get_string("prompt"),
            session,
            Some(ctx.file_path),
        )
    {
        return Some(reason);
    }
    let is_low_risk = session.intent_risk == "low";
    if !is_low_risk && !is_subagent && ctx.is_code && !session.evidence_window_satisfied() {
        return Some(format!(
            "EVIDENCE_CHAIN VIOLATION: no WebSearch since t{}. Search before implement.",
            session.intent_set_turn
        ));
    }
    None
}
