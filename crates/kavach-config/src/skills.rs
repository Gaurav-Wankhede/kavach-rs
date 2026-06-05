use crate::cache::load_patterns;
use crate::loaders::get_skill_patterns;
use std::collections::HashMap;

pub fn get_valid_skills() -> HashMap<String, bool> {
    let data = load_patterns("valid-skills.toon");
    let skills = data
        .get("VALID_SKILLS")
        .cloned()
        .unwrap_or_else(default_valid_skills);
    skills.into_iter().map(|s| (s, true)).collect()
}

pub fn default_valid_skills() -> Vec<String> {
    [
        "commit",
        "review-pr",
        "create-pr",
        "init",
        "status",
        "memory",
        "resume",
        "research",
        "plan",
        "debug-like-expert",
        "security",
        "frontend",
        "testing",
        "arch",
        "dsa",
        "sql",
        "api-design",
        "rust",
        "cloud-infrastructure-mastery",
        "high-performance-data-processing",
        "heal",
        "sutra-protocol",
        "create-claude-components",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SkillConfig {
    pub name: String,
    pub priority: i32,
    pub keywords: Vec<String>,
}

#[must_use]
pub fn get_skills_by_priority() -> Vec<SkillConfig> {
    let patterns = get_skill_patterns();
    let mut skills = Vec::new();
    for (section, values) in &patterns {
        let name = match section.strip_prefix("SKILL:") {
            Some(n) => n.to_owned(),
            None => continue,
        };
        let mut skill = SkillConfig {
            name,
            priority: 999,
            keywords: Vec::new(),
        };
        for v in values {
            if let Some(pri_str) = v.strip_prefix("priority:") {
                if let Ok(pri) = pri_str.trim().parse::<i32>() {
                    skill.priority = pri;
                }
            } else {
                skill.keywords.push(v.clone());
            }
        }
        skills.push(skill);
    }
    skills.sort_by_key(|s| s.priority);
    skills
}

#[must_use]
pub fn get_skill_names() -> Vec<String> {
    get_skills_by_priority()
        .into_iter()
        .map(|s| s.name)
        .collect()
}

#[must_use]
pub fn get_skill_keywords(skill_name: &str) -> Vec<String> {
    let patterns = get_skill_patterns();
    let section = format!("SKILL:{skill_name}");
    patterns
        .get(&section)
        .map(|vals| {
            vals.iter()
                .filter(|v| !v.starts_with("priority:"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_valid_skills() {
        let skills = default_valid_skills();
        assert!(skills.iter().any(|s| s == "commit"));
        assert!(skills.iter().any(|s| s == "rust"));
    }
    #[test]
    fn test_get_valid_skills_returns_defaults() {
        let skills = get_valid_skills();
        assert!(skills.contains_key("commit"));
        assert!(skills.contains_key("rust"));
    }
}
