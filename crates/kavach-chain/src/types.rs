use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VerificationResult {
    pub gate: String,
    pub status: String,
    pub reason: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IntentAnalysis {
    #[serde(rename = "type")]
    pub intent_type: String,
    pub confidence: f64,
    pub required_skills: Vec<String>,
    pub required_agents: Vec<String>,
    pub requires_research: bool,
    pub complexity: String,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CEODecision {
    pub approved: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub delegation_plan: String,
    pub assigned_agents: Vec<String>,
    pub task_breakdown: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AegisVerification {
    pub passed: bool,
    pub security_score: f64,
    pub threat_level: String,
    pub violations_found: Vec<String>,
    pub recommendations: Vec<String>,
    pub memory_provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ResearchStatus {
    pub done: bool,
    pub sources: Vec<String>,
    pub suggested_query: String,
    pub bypass: bool,
    pub bypass_reason: String,
}
