// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTier {
    Refactor,
    Feature,
    Platform,
}

impl ProjectTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refactor => "refactor",
            Self::Feature => "feature",
            Self::Platform => "platform",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        [Self::Refactor, Self::Feature, Self::Platform]
            .into_iter()
            .find(|tier| tier.as_str() == s)
    }

    #[must_use]
    pub const fn includes(self, min_tier: Self) -> bool {
        self.rank() >= min_tier.rank()
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Refactor => 1,
            Self::Feature => 2,
            Self::Platform => 3,
        }
    }

    #[must_use]
    pub const fn can_promote_to(self, target: Self) -> bool {
        target.rank() >= self.rank()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecCategory {
    AppSpec,
    Architecture,
    Roadmap,
}

impl SpecCategory {
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::AppSpec => "app_spec",
            Self::Architecture => "decision",
            Self::Roadmap => "roadmap",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactValidator {
    PrdShape,
    TrdShape,
    UserStoryShape,
    ApiContractShape,
    DataModelShape,
    UserFlowShape,
    MetricShape,
    StateFlowShape,
    SecurityShape,
    ObservabilityShape,
    NfrShape,
    DeployShape,
    RoadmapShape,
    UiFlowShape,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDraftSource {
    HumanOnly,
    CodebaseAst,
    GitLog,
    HandlerScan,
    TracingScan,
    RouteScan,
    TestScan,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredPrefix {
    pub point: u8,
    pub label: &'static str,
    pub category: SpecCategory,
    pub key_prefix: &'static str,
    pub min_tier: ProjectTier,
    pub validator: ArtifactValidator,
    pub auto_draftable: AutoDraftSource,
}

impl RequiredPrefix {
    #[must_use]
    pub const fn required_at(&self, tier: ProjectTier) -> bool {
        tier.includes(self.min_tier)
    }
}

pub const FOURTEEN_PREFIXES: [RequiredPrefix; 14] = [
    RequiredPrefix {
        point: 1,
        label: "PRD",
        category: SpecCategory::AppSpec,
        key_prefix: "spec.prd",
        min_tier: ProjectTier::Refactor,
        validator: ArtifactValidator::PrdShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
    RequiredPrefix {
        point: 2,
        label: "TRD",
        category: SpecCategory::Architecture,
        key_prefix: "arch.trd",
        min_tier: ProjectTier::Refactor,
        validator: ArtifactValidator::TrdShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
    RequiredPrefix {
        point: 5,
        label: "Data Model",
        category: SpecCategory::Architecture,
        key_prefix: "arch.data",
        min_tier: ProjectTier::Refactor,
        validator: ArtifactValidator::DataModelShape,
        auto_draftable: AutoDraftSource::CodebaseAst,
    },
    RequiredPrefix {
        point: 3,
        label: "UI/UX",
        category: SpecCategory::AppSpec,
        key_prefix: "ui.flow",
        min_tier: ProjectTier::Feature,
        validator: ArtifactValidator::UiFlowShape,
        auto_draftable: AutoDraftSource::RouteScan,
    },
    RequiredPrefix {
        point: 4,
        label: "User Flows",
        category: SpecCategory::AppSpec,
        key_prefix: "spec.user_flow",
        min_tier: ProjectTier::Feature,
        validator: ArtifactValidator::UserFlowShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
    RequiredPrefix {
        point: 6,
        label: "Impl Plan",
        category: SpecCategory::Roadmap,
        key_prefix: "roadmap.unit",
        min_tier: ProjectTier::Feature,
        validator: ArtifactValidator::RoadmapShape,
        auto_draftable: AutoDraftSource::GitLog,
    },
    RequiredPrefix {
        point: 8,
        label: "Stories/Edges",
        category: SpecCategory::AppSpec,
        key_prefix: "spec.story",
        min_tier: ProjectTier::Feature,
        validator: ArtifactValidator::UserStoryShape,
        auto_draftable: AutoDraftSource::TestScan,
    },
    RequiredPrefix {
        point: 9,
        label: "API Contracts",
        category: SpecCategory::Architecture,
        key_prefix: "arch.api",
        min_tier: ProjectTier::Feature,
        validator: ArtifactValidator::ApiContractShape,
        auto_draftable: AutoDraftSource::HandlerScan,
    },
    RequiredPrefix {
        point: 7,
        label: "Success Metrics",
        category: SpecCategory::AppSpec,
        key_prefix: "spec.metric",
        min_tier: ProjectTier::Platform,
        validator: ArtifactValidator::MetricShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
    RequiredPrefix {
        point: 10,
        label: "State Flow",
        category: SpecCategory::Architecture,
        key_prefix: "arch.state",
        min_tier: ProjectTier::Platform,
        validator: ArtifactValidator::StateFlowShape,
        auto_draftable: AutoDraftSource::HandlerScan,
    },
    RequiredPrefix {
        point: 11,
        label: "Security",
        category: SpecCategory::Architecture,
        key_prefix: "arch.security",
        min_tier: ProjectTier::Platform,
        validator: ArtifactValidator::SecurityShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
    RequiredPrefix {
        point: 12,
        label: "Observability",
        category: SpecCategory::Architecture,
        key_prefix: "arch.obs",
        min_tier: ProjectTier::Platform,
        validator: ArtifactValidator::ObservabilityShape,
        auto_draftable: AutoDraftSource::TracingScan,
    },
    RequiredPrefix {
        point: 13,
        label: "NFRs",
        category: SpecCategory::Architecture,
        key_prefix: "arch.nfr",
        min_tier: ProjectTier::Platform,
        validator: ArtifactValidator::NfrShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
    RequiredPrefix {
        point: 14,
        label: "Deploy/Rollout",
        category: SpecCategory::Architecture,
        key_prefix: "arch.deploy",
        min_tier: ProjectTier::Platform,
        validator: ArtifactValidator::DeployShape,
        auto_draftable: AutoDraftSource::HumanOnly,
    },
];

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeMode {
    pub project_slug: String,
    pub started_at_unix_s: i64,
    pub expires_at_unix_s: i64,
    pub reason: String,
}

impl SpikeMode {
    #[must_use]
    pub const fn is_active(&self, now_unix_s: i64) -> bool {
        now_unix_s < self.expires_at_unix_s
    }
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingPrefix {
    pub point: u8,
    pub label: String,
    pub key_prefix: String,
    pub reason: MissingReason,
    pub auto_draftable: AutoDraftSource,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    NoRows,
    ShapeInvalid { details: String },
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessResult {
    pub project_slug: String,
    pub tier: ProjectTier,
    pub present: u8,
    pub required: u8,
    pub missing: Vec<MissingPrefix>,
}

impl WitnessResult {
    #[must_use]
    pub const fn is_clear(&self) -> bool {
        self.missing.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_includes_lower_bars() {
        assert!(ProjectTier::Platform.includes(ProjectTier::Refactor));
        assert!(ProjectTier::Platform.includes(ProjectTier::Feature));
        assert!(ProjectTier::Feature.includes(ProjectTier::Refactor));
        assert!(!ProjectTier::Refactor.includes(ProjectTier::Feature));
        assert!(!ProjectTier::Feature.includes(ProjectTier::Platform));
    }

    #[test]
    fn tier_promotion_one_way() {
        assert!(ProjectTier::Refactor.can_promote_to(ProjectTier::Feature));
        assert!(ProjectTier::Refactor.can_promote_to(ProjectTier::Platform));
        assert!(ProjectTier::Feature.can_promote_to(ProjectTier::Platform));
        assert!(!ProjectTier::Feature.can_promote_to(ProjectTier::Refactor));
        assert!(!ProjectTier::Platform.can_promote_to(ProjectTier::Feature));
        assert!(!ProjectTier::Platform.can_promote_to(ProjectTier::Refactor));
    }

    #[test]
    fn tier_required_counts() {
        let refactor_count = FOURTEEN_PREFIXES
            .iter()
            .filter(|p| p.required_at(ProjectTier::Refactor))
            .count();
        let feature_count = FOURTEEN_PREFIXES
            .iter()
            .filter(|p| p.required_at(ProjectTier::Feature))
            .count();
        let platform_count = FOURTEEN_PREFIXES
            .iter()
            .filter(|p| p.required_at(ProjectTier::Platform))
            .count();
        assert_eq!(refactor_count, 3);
        assert_eq!(feature_count, 8);
        assert_eq!(platform_count, 14);
    }

    #[test]
    fn fourteen_prefixes_have_unique_points() {
        let mut points: Vec<u8> = FOURTEEN_PREFIXES.iter().map(|p| p.point).collect();
        points.sort_unstable();
        let unique = {
            let mut v = points.clone();
            v.dedup();
            v
        };
        assert_eq!(points.len(), unique.len());
        assert_eq!(points.len(), 14);
    }

    #[test]
    fn fourteen_prefixes_have_unique_key_prefixes() {
        let mut keys: Vec<&str> = FOURTEEN_PREFIXES.iter().map(|p| p.key_prefix).collect();
        keys.sort_unstable();
        let n_before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n_before);
    }

    #[test]
    fn tier_parse_round_trip() {
        for tier in [
            ProjectTier::Refactor,
            ProjectTier::Feature,
            ProjectTier::Platform,
        ] {
            assert_eq!(ProjectTier::parse(tier.as_str()), Some(tier));
        }
        assert_eq!(ProjectTier::parse("bogus"), None);
    }

    #[test]
    fn spike_mode_expires() {
        let s = SpikeMode {
            project_slug: "x".into(),
            started_at_unix_s: 1000,
            expires_at_unix_s: 2000,
            reason: "test".into(),
        };
        assert!(s.is_active(1500));
        assert!(!s.is_active(2000));
        assert!(!s.is_active(3000));
    }

    #[test]
    fn witness_clear_when_no_missing() {
        let w = WitnessResult {
            project_slug: "x".into(),
            tier: ProjectTier::Refactor,
            present: 3,
            required: 3,
            missing: vec![],
        };
        assert!(w.is_clear());
    }
}
