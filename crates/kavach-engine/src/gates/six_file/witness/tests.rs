//! `run_witness` coverage: all-clear, missing row, invalid shape, full tier.
use kavach_types::{MissingReason, ProjectTier};

use super::run_witness;

#[test]
fn test_refactor_all_clear() {
    let rows = vec![
        (
            "spec.prd.v1".to_owned(),
            "problem: X user: Y goal: Z constraint: W success: V".to_owned(),
        ),
        ("arch.trd.v1".to_owned(), "architecture document".to_owned()),
        (
            "arch.data.v1".to_owned(),
            "table User { id, name }".to_owned(),
        ),
    ];
    let result = run_witness(&rows, "test-proj", ProjectTier::Refactor);
    assert!(result.is_clear());
    assert_eq!(result.present, 3);
    assert_eq!(result.required, 3);
}

#[test]
fn test_refactor_missing_prd() {
    let rows = vec![
        ("arch.trd.v1".to_owned(), "architecture".to_owned()),
        ("arch.data.v1".to_owned(), "table User".to_owned()),
    ];
    let result = run_witness(&rows, "test-proj", ProjectTier::Refactor);
    assert!(!result.is_clear());
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0].label, "PRD");
}

#[test]
fn test_shape_invalid() {
    let rows = vec![
        ("spec.prd.v1".to_owned(), "just some text".to_owned()),
        ("arch.trd.v1".to_owned(), "architecture".to_owned()),
        ("arch.data.v1".to_owned(), "table User".to_owned()),
    ];
    let result = run_witness(&rows, "test-proj", ProjectTier::Refactor);
    assert!(!result.is_clear());
    let prd_missing = result.missing.iter().find(|m| m.label == "PRD").unwrap();
    assert!(matches!(
        prd_missing.reason,
        MissingReason::ShapeInvalid { .. }
    ));
}

#[test]
fn test_platform_tier() {
    let rows = vec![
        (
            "spec.prd.v1".to_owned(),
            "problem: X user: Y goal: Z constraint: W success: V".to_owned(),
        ),
        ("arch.trd.v1".to_owned(), "architecture".to_owned()),
        ("arch.data.v1".to_owned(), "table User".to_owned()),
        ("ui.flow.v1".to_owned(), "screen: login".to_owned()),
        (
            "spec.user_flow.v1".to_owned(),
            "flow: user logs in".to_owned(),
        ),
        ("roadmap.unit.v1".to_owned(), "goal: X verify: Y".to_owned()),
        (
            "spec.story.v1".to_owned(),
            "as a user i want to login so that".to_owned(),
        ),
        (
            "arch.api.v1".to_owned(),
            "POST /login status 200".to_owned(),
        ),
    ];
    let result = run_witness(&rows, "test-proj", ProjectTier::Platform);
    assert_eq!(result.required, 14);
}
