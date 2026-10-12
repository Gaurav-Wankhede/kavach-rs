//! Skill metadata and definition

use super::section::ResearchGate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal construction DTO"
)]
pub struct SkillDefinition {
    pub metadata: SkillMetadata,
    pub research_gate: ResearchGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal construction DTO"
)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub protocol: String,
    pub triggers: Vec<String>,
    #[serde(default)]
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub priority: super::SkillPriority,
}
