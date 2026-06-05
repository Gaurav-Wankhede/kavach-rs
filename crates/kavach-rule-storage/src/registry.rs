//! Cached skill registry for fast file-pattern lookups.

use kavach_rule_ast::SkillPriority;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RegistryEntry {
    pub name: String,
    pub file_patterns: Vec<String>,
    pub priority: SkillPriority,
}

impl RegistryEntry {
    #[must_use]
    pub const fn new(name: String, file_patterns: Vec<String>, priority: SkillPriority) -> Self {
        Self {
            name,
            file_patterns,
            priority,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SkillRegistry {
    pub version: u32,
    pub hash: String,
    pub built_at: String,
    pub skills: Vec<RegistryEntry>,
}

impl SkillRegistry {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            version: 1,
            hash: String::new(),
            built_at: String::new(),
            skills: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_entry_creation() {
        let entry = RegistryEntry::new(
            "rust-safety".into(),
            vec!["*.rs".into(), "Cargo.toml".into()],
            SkillPriority::Critical,
        );
        assert_eq!(entry.name, "rust-safety");
        assert_eq!(entry.file_patterns.len(), 2);
        assert!(entry.priority.is_critical());
    }

    #[test]
    fn test_empty_registry() {
        let reg = SkillRegistry::empty();
        assert_eq!(reg.version, 1);
        assert!(reg.skills.is_empty());
        assert!(reg.hash.is_empty());
        assert!(reg.built_at.is_empty());
    }

    #[test]
    fn test_registry_serialization_roundtrip() {
        let reg = SkillRegistry {
            version: 1,
            hash: "abc123".into(),
            built_at: "2026-03-14T00:00:00".into(),
            skills: vec![RegistryEntry {
                name: "sp-rust".into(),
                file_patterns: vec!["*.rs".into()],
                priority: SkillPriority::Advisory,
            }],
        };
        let json = serde_json::to_string(&reg).expect("SkillRegistry serialization failed");
        let back: SkillRegistry =
            serde_json::from_str(&json).expect("SkillRegistry deserialization failed");
        assert_eq!(back.version, reg.version);
        assert_eq!(back.hash, reg.hash);
        assert_eq!(back.skills.len(), 1);
        assert_eq!(
            back.skills.first().map(|e| &e.name),
            Some(&"sp-rust".to_owned())
        );
    }
}
