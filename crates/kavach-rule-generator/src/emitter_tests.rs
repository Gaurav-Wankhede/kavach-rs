//! Tests for emitter module.

use super::*;
use crate::detector::{DetectedPattern, PatternType};
use crate::template::generate_skill;

fn make_pattern() -> DetectedPattern {
    DetectedPattern {
        pattern_type: PatternType::Framework,
        name: "axum".into(),
        confidence: 0.9,
        evidence: vec!["matched: use axum".into()],
    }
}

#[test]
fn test_emit_skill_has_frontmatter() {
    let skill = generate_skill(&make_pattern());
    let output = emit_skill(&skill);
    assert!(output.starts_with("---\n"), "should start with frontmatter");
    assert!(output.contains("protocol: SP/3.0"));
    assert!(output.contains("name: axum"));
}

#[test]
fn test_emit_skill_has_skill_block() {
    let skill = generate_skill(&make_pattern());
    let output = emit_skill(&skill);
    assert!(output.contains("SKILL:axum"));
    assert!(output.contains("triggers:"));
}

#[test]
fn test_emit_skill_has_research_gate() {
    let skill = generate_skill(&make_pattern());
    let output = emit_skill(&skill);
    assert!(output.contains("RESEARCH_GATE"));
    assert!(output.contains("mandatory: true"));
}

#[test]
fn test_emit_full_skill_has_all_sections() {
    let pat = make_pattern();
    let skill = generate_skill(&pat);
    let output = emit_full_skill(&skill, &pat);
    assert!(output.contains("---\n"));
    assert!(output.contains("SKILL:axum"));
    assert!(output.contains("RESEARCH_GATE"));
    assert!(output.contains("ERROR_HANDLING"));
    assert!(output.contains("PENDING_TASKS"));
    assert!(output.contains("production_style:"));
}

#[test]
fn test_emit_full_skill_rust_error_handling() {
    let pat = DetectedPattern {
        pattern_type: PatternType::Language,
        name: "rust".into(),
        confidence: 0.9,
        evidence: vec![],
    };
    let skill = generate_skill(&pat);
    let output = emit_full_skill(&skill, &pat);
    assert!(output.contains("Result<T, E>"));
}
