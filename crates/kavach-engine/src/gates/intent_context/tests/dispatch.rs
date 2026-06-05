//! `append_agent_dispatch` routing-matrix tests.
use crate::gates::intent_context::directives::append_agent_dispatch;

#[test]
fn test_agent_dispatch_debug_routes_to_ceo_and_bug_bounty() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "debug");
    assert!(ctx.contains("INVOKE_AGENT: ceo"));
    assert!(ctx.contains("INVOKE_SKILL: bug-bounty"));
}

#[test]
fn test_agent_dispatch_refactor_routes_to_aegis_and_rust() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "refactor");
    assert!(ctx.contains("INVOKE_AGENT: aegis-guardian"));
    assert!(ctx.contains("INVOKE_SKILL: rust"));
}

#[test]
fn test_agent_dispatch_implement_invokes_writing_plans() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "implement");
    assert!(ctx.contains("INVOKE_SKILL: writing-plans"));
    assert!(ctx.contains("iteration-start"));
}

#[test]
fn test_agent_dispatch_general_invokes_research_director() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "general");
    assert!(ctx.contains("INVOKE_AGENT: research-director"));
}

#[test]
fn test_agent_dispatch_skipped_for_memory() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "memory");
    assert!(ctx.is_empty());
}

#[test]
fn test_agent_dispatch_skipped_for_unknown() {
    let mut ctx = String::new();
    append_agent_dispatch(&mut ctx, "frobnicate");
    assert!(ctx.is_empty());
}
