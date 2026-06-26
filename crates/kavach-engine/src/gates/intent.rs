//! Intent gate: classify user prompt, set enforcement state, route to skills.
//! Triggered by the `UserPromptSubmit` hook event.
mod classify;
mod context;
mod decision_map;
mod harness;
mod kvs;
mod pattern_dag;
mod phase;
mod practice_delta;
mod recall;
mod version_pin;

#[cfg(test)]
mod rag_tests;
#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;

use kavach_types::HookInput;

// Re-export so `session_start` can inject the SAME live `[KANBAN]` block at
// SessionStart that the UserPromptSubmit hook injects — both read the live board.
pub(in crate::gates) use context::append_live_kanban_block;
// SINGLE canonical emitter of the DECISION_MAP/PRACTICE_DELTA/PATTERN_DAG triad —
// both this hook and SessionStart call it, so the triad can never drift out of one.
pub(in crate::gates) use context::append_mermaid_views;

use super::intent_context::extract_research_topic;
use classify::{
    apply_focus_marker, collapse_required_via_rag, filter_invocable_skills, prompt_injection_block,
};
use context::append_context_blocks;
use kvs::build_base_context;

use crate::error::EngineError;

/// Intent gate entry point. Classifies the prompt, mutates session enforcement
/// state, and emits the `[INTENT]` context block.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by run_gate dispatch table: every gate handler returns Result<(), EngineError>"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let prompt = input.get_string("prompt");
    if prompt.is_empty() {
        drop(kavach_hook::exit_prompt_context(""));
        return Ok(());
    }
    // P0 SECURITY: block prompt injection BEFORE any other processing.
    if let Some(msg) = prompt_injection_block(prompt) {
        drop(kavach_hook::exit_prompt_submit_block(&msg));
        return Ok(());
    }

    let mut session = kavach_session::get_or_create_session();
    session.reset_research_for_new_prompt();
    session.reset_evidence_window();
    session.increment_turn();
    // Stamp this turn as USER-DIRECTED: a real `UserPromptSubmit` is the user
    // speaking (the autonomous loop re-injects via the Stop hook, not here). The
    // stop gate reads this to grant the user-focus override — a turn the user just
    // steered is NOT hijacked onto a different kanban card by the dispatcher.
    session.mark_user_directive();
    apply_focus_marker(&mut session, prompt);

    let intent = kavach_chain::analyze_intent(prompt);
    // Authoritatively write the freshly-derived classification back NOW: it is a
    // pure function of THIS prompt, but the parse.rs load-guard refuses to
    // overwrite a non-empty persisted value, which would latch turn-1's class.
    session.set_intent_type(&intent.intent_type);
    let skills_raw =
        collapse_required_via_rag(intent.required_skills.clone(), prompt, &intent.intent_type);
    let skills = filter_invocable_skills(skills_raw);
    session.store_intent(&intent.intent_type, &intent.complexity, skills.clone());
    if !skills.is_empty() {
        session.set_required_skills(skills);
    }

    if kavach_session::SessionState::detect_new_crate_confirmation(prompt) {
        session.confirm_new_crate();
    }
    // Internet-first: fire a background web-search NOW (hook budget forbids blocking).
    let research_pending = intent.requires_research.then(|| {
        let topic = extract_research_topic(prompt, &intent.intent_type);
        session.set_research_topic(&topic);
        kavach_advisor::clear(&session.session_id);
        kavach_advisor::kickoff(&session.session_id, &topic);
        topic
    });
    session.set_intent_risk(&intent.risk_level);
    super::event_log::log_intent(
        &session.session_id,
        &intent.intent_type,
        &intent.risk_level,
        &session.project,
    );

    // L4: classify the prompt into a harness pattern + persist it on the next
    // card so the L3 stop-gate dispatches that workflow. Fail-soft + advisory.
    let harness_block = harness::persist_for_next_card(&session.project, prompt);

    let router = kavach_chain::SkillFirstRouter::new();
    let keywords: Vec<&str> = prompt.split_whitespace().collect();
    let routing = router.route(prompt, &keywords);
    let forbidden = kavach_chain::ResearchGate::new().check_forbidden_phrases(prompt);

    let mut context = build_base_context(&intent, &routing, &session);
    // Pin research to installed versions: hand the LLM exact Cargo.lock versions of
    // any dependency named in the prompt, so a query can never drift to stale weights.
    context.push_str(&version_pin::version_pin_block(prompt));
    if let Some(topic) = research_pending {
        let pending = format!(
            "\n[RESEARCH:PENDING] topic={topic} — internet-first lookup dispatched. \
             Findings arrive in the turn cache; the pre-write gate BLOCKS edits until \
             you cite a source URL or [RESEARCH] block.\n"
        );
        context.push_str(&pending);
    }
    context.push_str(&harness_block);
    // Brain-OS auto-recall: consult memory on every prompt (fail-soft, advisory).
    context.push_str(&recall::recall_block(prompt));
    append_context_blocks(
        &mut context,
        input,
        &mut session,
        &intent.intent_type,
        prompt,
        &forbidden,
    );

    // Cursor turn shadow: persist compact per-turn context for preToolUse relay.
    let harness_pattern = harness::classify_harness(prompt);
    let top_skill = super::rag_router::top_skill_names_all("", prompt, &intent.intent_type, 1)
        .into_iter()
        .next();
    let shadow = super::loop_frame::build_turn_shadow(
        &session,
        &intent,
        harness_pattern,
        top_skill.as_deref(),
    );
    session.store_turn_shadow(&shadow);

    drop(kavach_hook::exit_prompt_context(&context));
    Ok(())
}
