// TIME: O(1) lookup, O(n) build | SPACE: O(n) entries + O(k) triggers
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://docs.rs/aho-corasick (trigger matching pattern)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::skill_keyword_router::{ModelTier, SkillContext};

/// Lightweight skill entry without full content.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SkillManifestEntry {
    pub name: String,
    pub path: PathBuf,
    pub triggers: Vec<String>,
    pub context: SkillContext,
    pub agent: Option<String>,
    pub model_tier: ModelTier,
    pub description: Option<String>,
}

/// Manifest of all skills with just routing metadata.
/// Full SKILL.md content NOT loaded until `load_content()` called.
#[derive(Debug, Default)]
pub struct SkillManifest {
    entries: HashMap<String, SkillManifestEntry>,
    trigger_index: HashMap<String, String>,
}

fn extract_frontmatter(content: &str) -> Option<Vec<&str>> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first() != Some(&"---") {
        return None;
    }
    let end_idx = lines.iter().skip(1).position(|l| *l == "---")?;
    lines.get(1..=end_idx).map(<[&str]>::to_vec)
}

fn extract_triggers(content: &str) -> Vec<String> {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return Vec::new();
    };
    for line in frontmatter {
        if line.trim().starts_with("triggers:") {
            let Some(start) = line.find('[') else {
                continue;
            };
            let Some(end) = line.find(']') else { continue };
            let start_idx = start.saturating_add(1);
            if start_idx >= end {
                continue;
            }
            let arr = &line.get(start_idx..end).unwrap_or("");
            return arr
                .split(',')
                .map(|kw| kw.trim().trim_matches('"').trim_matches('\'').to_owned())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    Vec::new()
}

fn extract_skill_metadata(content: &str) -> (SkillContext, Option<String>, ModelTier) {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return (SkillContext::default(), None, ModelTier::default());
    };
    let mut skill_context = SkillContext::Inline;
    let mut agent = None;
    let mut tier = ModelTier::Sonnet;
    for line in frontmatter {
        let trimmed = line.trim();
        if trimmed.starts_with("context:") {
            let val = trimmed.strip_prefix("context:").unwrap_or("").trim();
            if val == "fork" {
                skill_context = SkillContext::Fork;
            }
        }
        if trimmed.starts_with("agent:") {
            let val = trimmed.strip_prefix("agent:").unwrap_or("").trim();
            if !val.is_empty() {
                agent = Some(val.to_owned());
            }
        }
        if trimmed.starts_with("model_tier:") {
            let val = trimmed
                .strip_prefix("model_tier:")
                .unwrap_or("")
                .trim()
                .to_lowercase();
            tier = match val.as_str() {
                "haiku" => ModelTier::Haiku,
                "opus" => ModelTier::Opus,
                _ => ModelTier::Sonnet,
            };
        }
    }
    (skill_context, agent, tier)
}

fn extract_description(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("description:") {
            let desc = trimmed.strip_prefix("description:").unwrap_or("").trim();
            let desc = desc.trim_matches('"').trim_matches('\'');
            if !desc.is_empty() {
                return Some(desc.to_owned());
            }
        }
    }
    None
}

impl SkillManifest {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn skills_dir() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".claude/skills"))
            .unwrap_or_default()
    }

    /// Build manifest by scanning SKILL.md frontmatter only.
    #[must_use]
    pub fn build() -> Self {
        Self::build_from(&Self::skills_dir())
    }

    /// Build a manifest from a specific skills directory. Split from `build` so
    /// tests can scan a hermetic fixture dir instead of the developer's real
    /// `~/.claude/skills` (absent on CI runners → empty manifest → false test
    /// failures). SOURCE: rca.non-hermetic-skill-router-tests.
    #[must_use]
    pub fn build_from(dir: &Path) -> Self {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Self::default();
        };

        let mut manifest = Self::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let skill_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_owned(),
                None => continue,
            };

            let skill_md = path.join("SKILL.md");
            let Ok(file_content) = std::fs::read_to_string(&skill_md) else {
                continue;
            };

            let triggers = extract_triggers(&file_content);
            if triggers.is_empty() {
                continue;
            }

            let (skill_ctx, agent, model_tier) = extract_skill_metadata(&file_content);
            let description = extract_description(&file_content.clone());

            let manifest_entry = SkillManifestEntry {
                name: skill_name.clone(),
                path: skill_md,
                triggers: triggers.clone(),
                context: skill_ctx,
                agent,
                model_tier,
                description,
            };

            for trigger in &triggers {
                manifest
                    .trigger_index
                    .insert(trigger.to_lowercase(), skill_name.clone());
            }
            manifest.entries.insert(skill_name, manifest_entry);
        }

        manifest
    }

    /// Get skill name for a trigger keyword. O(1) lookup.
    pub fn skill_for_trigger(&self, keyword: &str) -> Option<&str> {
        self.trigger_index
            .get(&keyword.to_lowercase())
            .map(String::as_str)
    }

    /// Get manifest entry by skill name. O(1) lookup.
    #[must_use]
    pub fn get(&self, skill_name: &str) -> Option<&SkillManifestEntry> {
        self.entries.get(skill_name)
    }

    /// List all skill names.
    pub fn skill_names(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }

    /// Get skills matching any of the given keywords.
    #[must_use]
    pub fn match_keywords(&self, keywords: &[&str]) -> Vec<&str> {
        let mut matched = Vec::new();
        for kw in keywords {
            if let Some(skill) = self.skill_for_trigger(kw)
                && !matched.contains(&skill)
            {
                matched.push(skill);
            }
        }
        matched
    }

    /// Load full SKILL.md content for a skill. Called only when skill is invoked.
    #[must_use]
    pub fn load_content(&self, skill_name: &str) -> Option<String> {
        let entry = self.entries.get(skill_name)?;
        std::fs::read_to_string(&entry.path).ok()
    }

    /// Get summary line for a skill (for context-efficient listing).
    #[must_use]
    pub fn summary_line(&self, skill_name: &str) -> Option<String> {
        let entry = self.entries.get(skill_name)?;
        let desc = entry.description.as_deref().unwrap_or("(no description)");
        let triggers = entry.triggers.join(", ");
        Some(format!("{skill_name}: {desc} [{triggers}]"))
    }

    /// Total token budget estimate for manifest (names + triggers only).
    #[must_use]
    pub fn manifest_token_estimate(&self) -> usize {
        let total_chars = self
            .entries
            .values()
            .map(|e| {
                e.name
                    .len()
                    .saturating_add(e.triggers.iter().map(String::len).sum::<usize>())
            })
            .sum::<usize>();
        #[expect(
            clippy::integer_division,
            reason = "intentional truncation for token estimate"
        )]
        {
            total_chars / 4
        }
    }
}

/// Global cached manifest.
pub fn manifest() -> &'static SkillManifest {
    static MANIFEST: OnceLock<SkillManifest> = OnceLock::new();
    MANIFEST.get_or_init(SkillManifest::build)
}

/// Check if a skill exists in manifest.
#[must_use]
pub fn skill_exists(name: &str) -> bool {
    manifest().get(name).is_some()
}



#[cfg(test)]
mod tests {
    use super::*;

    /// Build a hermetic fixture skills dir with two SKILL.md files, so the test
    /// asserts the scanner populates a manifest without depending on the
    /// developer's real `~/.claude/skills` (absent on CI runners).
    /// SOURCE: rca.non-hermetic-skill-router-tests.
    fn fixture_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, triggers) in [("arch", r#"["architecture"]"#), ("data", r#"["sqlx"]"#)] {
            let skill_dir = dir.path().join(name);
            std::fs::create_dir_all(&skill_dir).expect("mkdir");
            let body = format!("---\nname: {name}\ntriggers: {triggers}\n---\n# {name}\n");
            std::fs::write(skill_dir.join("SKILL.md"), body).expect("write");
        }
        dir
    }

    #[test]
    fn manifest_builds_without_panic() {
        let dir = fixture_dir();
        let m = SkillManifest::build_from(dir.path());
        assert!(
            !m.entries.is_empty(),
            "Expected the scanner to populate the manifest from the fixture dir"
        );
    }

    #[test]
    fn manifest_token_estimate_reasonable() {
        let m = manifest();
        let tokens = m.manifest_token_estimate();
        assert!(
            tokens < 5000,
            "Manifest should be <5000 tokens, got {tokens}"
        );
    }

    #[test]
    fn load_content_returns_full_skill() {
        let m = manifest();
        if let Some(name) = m.skill_names().first() {
            let content = m.load_content(name);
            assert!(content.is_some(), "Should load content for {name}");
            assert!(
                content.unwrap().contains("---"),
                "Content should have frontmatter"
            );
        }
    }
}
