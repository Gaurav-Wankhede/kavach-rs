//! Tests for template module.

use super::*;
use crate::detector::{DetectedPattern, PatternType};

fn rust_pattern() -> DetectedPattern {
    DetectedPattern {
        pattern_type: PatternType::Language,
        name: "rust".into(),
        confidence: 0.9,
        evidence: vec!["10 .rs files found".into()],
    }
}

fn axum_pattern() -> DetectedPattern {
    DetectedPattern {
        pattern_type: PatternType::Framework,
        name: "axum".into(),
        confidence: 0.95,
        evidence: vec!["matched: use axum".into()],
    }
}

#[test]
fn test_generate_rust_skill() {
    let skill = generate_skill(&rust_pattern());
    assert_eq!(skill.metadata.name, "rust");
    assert_eq!(skill.metadata.protocol, "SP/3.0");
    assert!(skill.research_gate.mandatory);
}

#[test]
fn test_generate_framework_skill() {
    let skill = generate_skill(&axum_pattern());
    assert!(skill.metadata.description.contains("framework"));
    assert!(skill.metadata.triggers.contains(&"axum".to_owned()));
}

#[test]
fn test_error_handling_rust() {
    let eh = generate_error_handling(&rust_pattern());
    assert!(eh.production_style.contains("Result<T, E>"));
    assert!(eh.test_only.contains(&"unwrap()".to_owned()));
}

#[test]
fn test_error_handling_go() {
    let go = DetectedPattern {
        pattern_type: PatternType::Language,
        name: "go".into(),
        confidence: 0.8,
        evidence: vec![],
    };
    let eh = generate_error_handling(&go);
    assert!(eh.production_style.contains("err != nil"));
}

#[test]
fn test_pending_tasks_language() {
    let pt = generate_pending_tasks(&rust_pattern());
    assert!(pt.mandatory);
    assert_eq!(pt.macros.len(), 2);
    assert!(pt.macros.first().is_some_and(|m| m.contains("rust")));
}

#[test]
fn test_pending_tasks_tool() {
    let docker = DetectedPattern {
        pattern_type: PatternType::Tool,
        name: "docker".into(),
        confidence: 0.7,
        evidence: vec![],
    };
    let pt = generate_pending_tasks(&docker);
    assert!(pt.macros.first().is_some_and(|m| m.contains("docker")));
}
