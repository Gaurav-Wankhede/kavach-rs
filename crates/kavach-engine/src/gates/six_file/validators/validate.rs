//! The artifact-shape dispatch table: maps each `ArtifactValidator` to its
//! keyword check (helpers) or compound predicate (shapes).
use kavach_types::ArtifactValidator;

use super::helpers::{has_any, has_min_signals};
use super::shapes::{api_contract, roadmap, state_flow, user_story};

pub(crate) fn validate(validator: ArtifactValidator, content: &str) -> Result<(), String> {
    let lower = content.to_lowercase();
    match validator {
        ArtifactValidator::PrdShape => has_min_signals(
            &lower,
            &["problem", "user", "goal", "constraint", "success"],
            3,
            "PRD",
        ),
        ArtifactValidator::TrdShape => {
            has_any(&lower, &["architecture", "design", "system"], "TRD")
        }
        ArtifactValidator::UserStoryShape => user_story(&lower),
        ArtifactValidator::ApiContractShape => api_contract(&lower),
        ArtifactValidator::DataModelShape => has_any(
            &lower,
            &["table", "struct", "field", "schema", "entity"],
            "data model",
        ),
        ArtifactValidator::UserFlowShape => has_any(
            &lower,
            &["flow", "step", "screen", "user", "action"],
            "user flow",
        ),
        ArtifactValidator::MetricShape => has_any(
            &lower,
            &["metric", "kpi", "success", "measure", "target"],
            "metric",
        ),
        ArtifactValidator::StateFlowShape => state_flow(&lower),
        ArtifactValidator::SecurityShape => has_any(
            &lower,
            &["security", "auth", "threat", "permission", "access"],
            "security",
        ),
        ArtifactValidator::ObservabilityShape => has_any(
            &lower,
            &["metric", "trace", "log", "monitor", "observ"],
            "observability",
        ),
        ArtifactValidator::NfrShape => has_any(
            &lower,
            &[
                "performance",
                "scalability",
                "reliability",
                "latency",
                "throughput",
            ],
            "NFR",
        ),
        ArtifactValidator::DeployShape => has_any(
            &lower,
            &["deploy", "release", "rollout", "staging", "production"],
            "deploy",
        ),
        ArtifactValidator::RoadmapShape => roadmap(&lower),
        ArtifactValidator::UiFlowShape => has_any(
            &lower,
            &["ui", "screen", "button", "form", "component"],
            "UI flow",
        ),
        // ArtifactValidator is #[non_exhaustive]: a variant added upstream but unknown
        // to this binary passes rather than spuriously blocking a valid artifact.
        _ => Ok(()),
    }
}
