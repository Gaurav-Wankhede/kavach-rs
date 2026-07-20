//! `append_agent_dispatch` routing tests.
//!
//! These exercise the STATIC fallback table: an empty prompt yields no
//! rankable words, so `try_dynamic_dispatch` returns false and the intent-keyed
//! default is used. The dynamic path is covered in `dynamic.rs`.
use crate::gates::intent_context::directives::{append_agent_dispatch, append_diagram_first};

#[test]
fn diagram_first_fires_for_refactor_intent() {
    let mut ctx = String::new();
    append_diagram_first(&mut ctx, "refactor", "");
    assert!(ctx.contains("[DIAGRAM_FIRST]"));
    assert!(ctx.contains("Mermaid"));
    assert!(ctx.contains("HTML"));
}

#[test]
fn diagram_first_fires_for_implement_intent() {
    // An implement turn that proposes architecture/LLD must show the diagram too.
    let mut ctx = String::new();
    append_diagram_first(&mut ctx, "implement", "");
    assert!(ctx.contains("[DIAGRAM_FIRST]"));
}

#[test]
fn diagram_first_fires_for_design_intent() {
    let mut ctx = String::new();
    append_diagram_first(&mut ctx, "design", "");
    assert!(ctx.contains("[DIAGRAM_FIRST]"));
}

#[test]
fn diagram_first_skipped_for_memory_intent() {
    let mut ctx = String::new();
    append_diagram_first(&mut ctx, "memory", "");
    assert!(ctx.is_empty());
}

#[test]
fn diagram_first_skipped_for_debug_without_keywords() {
    let mut ctx = String::new();
    append_diagram_first(&mut ctx, "debug", "fix the typo");
    assert!(ctx.is_empty());
}

#[test]
fn diagram_first_fires_for_debug_with_architecture_keyword() {
    let mut ctx = String::new();
    append_diagram_first(&mut ctx, "debug", "fix architecture issue");
    assert!(ctx.contains("[DIAGRAM_FIRST]"));
}

#[test]
fn test_agent_dispatch_debug_routes_to_ceo_and_bug_bounty() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "debug", "", "");
    assert!(ctx.contains("INVOKE_AGENT: ceo"));
    assert!(ctx.contains("INVOKE_SKILL: bug-bounty"));
}

#[test]
fn test_agent_dispatch_refactor_routes_to_aegis_and_rust() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "refactor", "", "");
    assert!(ctx.contains("INVOKE_AGENT: aegis-guardian"));
    assert!(ctx.contains("INVOKE_SKILL: rust"));
}

#[test]
fn test_agent_dispatch_implement_invokes_writing_plans() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "implement", "", "");
    assert!(ctx.contains("INVOKE_SKILL: writing-plans"));
    assert!(ctx.contains("iteration-start"));
}

#[test]
fn test_agent_dispatch_general_invokes_research_director() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "general", "", "");
    assert!(ctx.contains("INVOKE_AGENT: research-director"));
}

#[test]
fn dispatch_to_an_agent_carries_the_fanout_law() {
    for intent in ["debug", "refactor", "general"] {
        let mut ctx = String::new();
        append_agent_dispatch(&mut ctx, intent, "", "");
        assert!(
            ctx.contains("[FANOUT_LAW]"),
            "{intent}: an INVOKE_AGENT directive must carry the fan-out-to-cheap-tier law"
        );
        assert!(
            ctx.contains("claude-haiku-4-5"),
            "{intent}: the fan-out law must name the cheap executor tier"
        );
        assert!(
            ctx.contains("FIRST copy the source file with cp"),
            "{intent}: the fan-out law must include the migration copy-first rule"
        );
    }
}

#[test]
fn skill_only_dispatch_omits_the_fanout_law() {
    // `implement` routes to a SKILL, not an agent — no agent to fan out to.
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "implement", "", "");
    assert!(!ctx.contains("[FANOUT_LAW]"));
}

#[test]
fn test_agent_dispatch_skipped_for_memory() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "memory", "", "");
    assert!(ctx.is_empty());
}

#[test]
fn test_agent_dispatch_skipped_for_unknown() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "frobnicate", "", "");
    assert!(ctx.is_empty());
}
