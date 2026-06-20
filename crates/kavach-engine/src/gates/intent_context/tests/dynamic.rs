//! Dynamic-dispatch behaviour for `append_agent_dispatch`.
//!
//! The ranker reads the on-disk agent registry, which varies by environment, so
//! these tests assert INVARIANTS that hold regardless of which agents exist:
//! a known intent always yields SOME directive (dynamic OR static fallback),
//! and the research topic is folded into the dispatch decision without panic.

use crate::gates::intent_context::directives::append_agent_dispatch;

#[test]
fn known_intent_always_produces_a_directive() {
    // Whether the ranker fires or not, a known intent must never go silent —
    // the static fallback guarantees a directive. (Hybrid contract.)
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "debug", "fix the broken auth handler bug", "");
    assert!(!ctx.is_empty(), "known intent must always dispatch something");
    assert!(ctx.contains("INVOKE_AGENT"), "must name an agent");
}

#[test]
fn research_topic_is_folded_without_panic() {
    // A populated research topic must enrich the ranking query, not crash.
    let mut ctx = String::new();
    append_agent_dispatch(
        &mut ctx,
        "general",
        "investigate the database access pattern",
        "postgres row-level security 2026",
    );
    assert!(!ctx.is_empty());
}

#[test]
fn empty_prompt_falls_back_to_static_table() {
    // No rankable words ⇒ dynamic path returns false ⇒ static default used.
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "general", "", "");
    assert!(ctx.contains("research-director"), "empty prompt ⇒ static default");
}

#[test]
fn unknown_intent_empty_prompt_stays_silent() {
    // Neither dynamic (no words) nor static (no table entry) ⇒ no directive.
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "frobnicate", "", "");
    assert!(ctx.is_empty());
}
