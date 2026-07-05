//! Directive-builder tests: forbidden, RCA protocol, agent dispatch, research topic.
use crate::gates::intent_context::directives::{append_forbidden, append_root_cause_protocol};
use crate::gates::intent_context::research::extract_research_topic;

#[test]
fn test_append_forbidden_empty() {
    let mut ctx = String::new();
    append_forbidden(&mut ctx, &[]);
    assert!(ctx.is_empty());
}

#[test]
fn test_append_forbidden_items() {
    let mut ctx = String::new();
    append_forbidden(&mut ctx, &["bad phrase".into()]);
    assert!(ctx.contains("bad phrase"));
}

#[test]
fn test_extract_research_topic() {
    let topic = extract_research_topic("fix the auth bug in login", "debug");
    assert!(topic.contains("fix"));
}

#[test]
fn test_rca_protocol_injected_for_debug() {
    let mut ctx = String::new();
    append_root_cause_protocol(&mut ctx, "debug");
    assert!(ctx.contains("ROOT_CAUSE_PROTOCOL"));
    assert!(ctx.contains("Fix cause"));
    assert!(ctx.contains("[RCA]"));
    assert!(ctx.contains("blast_radius"));
    assert!(ctx.contains("why×5"));
}

#[test]
fn test_rca_protocol_injected_for_refactor() {
    let mut ctx = String::new();
    append_root_cause_protocol(&mut ctx, "refactor");
    assert!(ctx.contains("ROOT_CAUSE_PROTOCOL"));
}

#[test]
fn test_rca_protocol_skipped_for_general() {
    let mut ctx = String::new();
    append_root_cause_protocol(&mut ctx, "general");
    assert!(ctx.is_empty());
}

#[test]
fn test_rca_protocol_skipped_for_memory() {
    let mut ctx = String::new();
    append_root_cause_protocol(&mut ctx, "memory");
    assert!(ctx.is_empty());
}
