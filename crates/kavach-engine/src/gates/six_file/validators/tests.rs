//! Artifact-shape validator smoke tests (PRD, API contract, roadmap).
use kavach_types::ArtifactValidator;

use crate::gates::six_file::validators::validate;

#[test]
fn test_prd_valid() {
    let c = "problem: retention. user: mobile. goal: 30%. constraint: ios. success: shipped.";
    assert!(validate(ArtifactValidator::PrdShape, c).is_ok());
}

#[test]
fn test_api_valid() {
    let c = "POST /api/users → status 201";
    assert!(validate(ArtifactValidator::ApiContractShape, c).is_ok());
}

#[test]
fn test_roadmap_valid() {
    let c = "Goal: X. Verify: test >=80%.";
    assert!(validate(ArtifactValidator::RoadmapShape, c).is_ok());
}
