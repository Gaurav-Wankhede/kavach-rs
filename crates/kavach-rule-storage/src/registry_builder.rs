//! Build a `SkillRegistry` from loaded `StoredRules`.

use crate::registry::{RegistryEntry, SkillRegistry};
use crate::store::StoredRule;
use crate::version::RuleVersion;

/// Build registry from loaded rules, including only skills with `file_patterns`.
#[must_use]
pub fn build_from_rules(rules: &[StoredRule]) -> SkillRegistry {
    let skills: Vec<RegistryEntry> = rules
        .iter()
        .filter(|r| !r.definition.metadata.file_patterns.is_empty())
        .map(|r| RegistryEntry {
            name: r.definition.metadata.name.clone(),
            file_patterns: r.definition.metadata.file_patterns.clone(),
            priority: r.definition.metadata.priority.clone(),
        })
        .collect();

    let combined_hash = rules
        .iter()
        .map(|r| r.content_hash.as_str())
        .collect::<Vec<&str>>()
        .join(",");
    let hash = RuleVersion::compute_hash(&combined_hash);

    let built_at = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    SkillRegistry {
        version: 1,
        hash,
        built_at,
        skills,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_rule_ast::section::ResearchGate;
    use kavach_rule_ast::{SkillDefinition, SkillMetadata, SkillPriority};
    use std::path::PathBuf;

    fn make_rule(name: &str, file_patterns: Vec<String>) -> StoredRule {
        StoredRule {
            definition: SkillDefinition {
                metadata: SkillMetadata {
                    name: name.into(),
                    description: "test".into(),
                    protocol: "SP/1.0".into(),
                    triggers: vec![],
                    file_patterns,
                    priority: SkillPriority::Advisory,
                },
                research_gate: ResearchGate {
                    mandatory: false,
                    rule: String::new(),
                },
            },
            source_path: PathBuf::from(format!("/tmp/{name}.toon")),
            content_hash: format!("hash-{name}"),
            last_modified: "2026-03-14T00:00:00".into(),
            version: 1,
        }
    }

    #[test]
    fn test_build_from_stored_rules() {
        let rules = vec![
            make_rule("sp-rust", vec!["*.rs".into(), "Cargo.toml".into()]),
            make_rule("sp-generic", vec![]),
        ];
        let registry = build_from_rules(&rules);
        assert_eq!(registry.version, 1);
        assert_eq!(registry.skills.len(), 1);
        assert_eq!(
            registry.skills.first().map(|s| &s.name),
            Some(&"sp-rust".to_owned())
        );
        assert_eq!(
            registry.skills.first().map(|s| s.file_patterns.len()),
            Some(2)
        );
        assert!(!registry.hash.is_empty());
        assert!(!registry.built_at.is_empty());
    }

    #[test]
    fn test_build_empty_rules() {
        let registry = build_from_rules(&[]);
        assert_eq!(registry.version, 1);
        assert!(registry.skills.is_empty());
        assert!(!registry.hash.is_empty());
    }
}
