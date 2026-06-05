//! Section structures for skill TOON documents

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO constructed from kavach-rule-engine/storage/generator; non_exhaustive => E0639"
)]
pub struct ResearchGate {
    pub mandatory: bool,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO constructed from kavach-rule-generator; non_exhaustive => E0639"
)]
pub struct ErrorHandling {
    pub production_style: String,
    pub test_only: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO constructed from kavach-rule-generator; non_exhaustive => E0639"
)]
pub struct PendingTasks {
    pub mandatory: bool,
    pub macros: Vec<String>,
}
