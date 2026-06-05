use serde::Deserialize;
use std::collections::HashMap;

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GatesConfig {
    #[serde(rename = "$schema", default)]
    pub schema: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub read: ReadConfig,
    #[serde(default)]
    pub bash: BashConfig,
    #[serde(default)]
    pub write: WriteConfig,
    #[serde(default)]
    pub enforcer: EnforcerConfig,
    #[serde(default)]
    pub intent: IntentConfig,
    #[serde(default)]
    pub research: ResearchConfig,
    #[serde(default)]
    pub context: ContextConfig,
    #[serde(default)]
    pub quality: QualityConfig,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReadConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub blocked_paths: Vec<String>,
    #[serde(default)]
    pub blocked_extensions: Vec<String>,
    #[serde(default)]
    pub warn_extensions: Vec<String>,
    #[serde(default)]
    pub warn_patterns: Vec<String>,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BashConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub blocked_commands: Vec<String>,
    #[serde(default)]
    pub blocked_patterns: Vec<String>,
    #[serde(default)]
    pub warn_commands: Vec<String>,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WriteConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub blocked_paths: Vec<String>,
    #[serde(default)]
    pub protected_files: Vec<String>,
    #[serde(default)]
    pub secret_patterns: Vec<String>,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnforcerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub chain: Vec<String>,
    #[serde(default)]
    pub fail_fast: bool,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct IntentConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub skill_triggers: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub research_triggers: Vec<String>,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResearchConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub require_before_code: bool,
    #[serde(default)]
    pub code_tools: Vec<String>,
    #[serde(default)]
    pub research_tools: Vec<String>,
    #[serde(default)]
    pub bypass_patterns: Vec<String>,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContextConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub track_hot_paths: bool,
    #[serde(default)]
    pub max_hot_files: usize,
    #[serde(default)]
    pub persist_to_stm: bool,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct QualityConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub check_syntax: bool,
    #[serde(default)]
    pub check_imports: bool,
    #[serde(default)]
    pub max_file_size_kb: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_gates_config_json_deserialize() {
        let json = r#"{
            "$schema": "test",
            "read": { "enabled": true, "blocked_paths": ["/secret"] },
            "bash": { "enabled": false }
        }"#;
        let cfg: GatesConfig = serde_json::from_str(json).expect("parse");
        assert!(cfg.read.enabled);
        assert!(!cfg.bash.enabled);
        assert_eq!(cfg.read.blocked_paths, vec!["/secret"]);
    }
}
