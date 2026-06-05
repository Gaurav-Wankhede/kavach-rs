//! Trigger definitions for skills

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO deserialized from TOON; non_exhaustive => E0639"
)]
pub struct Trigger {
    pub name: String,
    pub category: TriggerCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate DTO deserialized from TOON; non_exhaustive => E0639"
)]
pub enum TriggerCategory {
    Language,
    Framework,
    Tool,
    Domain,
}
