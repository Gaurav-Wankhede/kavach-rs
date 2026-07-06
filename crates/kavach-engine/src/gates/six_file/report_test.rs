//! Block-report formatting: CLEAR status when nothing missing, and the
//! `[SIX_FILE_POLICY]` header + per-prefix line when artifacts are missing.
use super::format_block;
use kavach_types::{MissingPrefix, ProjectTier, WitnessResult};

#[test]
fn test_clear_report() {
    let result = WitnessResult {
        project_slug: "test".into(),
        tier: ProjectTier::Refactor,
        present: 3,
        required: 3,
        missing: vec![],
    };
    let report = format_block(&result);
    assert!(report.contains("CLEAR ✓"));
    assert!(report.contains("Spec coverage: 3/3"));
}

#[test]
fn test_missing_report() {
    let result = WitnessResult {
        project_slug: "test".into(),
        tier: ProjectTier::Refactor,
        present: 2,
        required: 3,
        missing: vec![MissingPrefix {
            point: 1,
            label: "PRD".into(),
            key_prefix: "spec.prd".into(),
            reason: kavach_types::MissingReason::NoRows,
            auto_draftable: kavach_types::AutoDraftSource::HumanOnly,
        }],
    };
    let report = format_block(&result);
    assert!(report.contains("[SIX_FILE_BLOCK]"));
    assert!(report.contains("✗ [1] PRD"));
}
