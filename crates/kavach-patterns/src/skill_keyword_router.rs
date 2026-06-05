use aho_corasick::AhoCorasick;
/// NLP keyword→skill routing using Aho-Corasick multi-pattern matching.
///
/// Dynamically loads keywords from ~/.claude/skills/*/SKILL.md frontmatter.
/// Single-pass O(n) scan of prompt text detects keywords that map to skills.
///
/// Supports `context: fork` and `agent: <type>` frontmatter for subagent routing.
/// Skills with `context: fork` should spawn as isolated subagents rather than
/// injecting into main context.
// ALGO: AhoCorasick
// PROBLEM_CLASS: string_match
// REJECTED: [{"name":"HashMap_per_word","reason":"O(k*n) per-word lookup, no single-pass"},{"name":"RegexAlternation","reason":"O(n*k) backtracking, cache-hostile"}]
// TIME: O(n+m+z) single pass | SPACE: O(Σ·k)
// YEAR: 1975 (Aho, Corasick) | SEARCHED: 2026-04
// TRADEOFF: filesystem read at startup — cached via OnceLock
// BENCHMARK: https://docs.rs/aho-corasick/latest/aho_corasick/
use std::sync::OnceLock;

/// Execution context for a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SkillContext {
    /// Inject skill into current conversation context (default).
    #[default]
    Inline,
    /// Spawn skill as isolated subagent, return only results.
    Fork,
}

/// Model tier required for a skill.
///
/// Used by gates to route the skill to an appropriately sized model when forked.
/// SOURCE: 42-pattern catalog §5.5 Model Tier Assignment.
/// SOURCE: benchlm.ai/blog/posts/claude-api-pricing — 2026 tier strategy
///   (Haiku $1/$5, Sonnet $3/$15, Opus $5/$25 per 1M in/out tokens).
///
/// Cost guideline (2026):
///   - Haiku: classification, routing, simple extraction (cheap, fast)
///   - Sonnet: implementation, code review, refactoring (default tier)
///   - Opus: architecture, security analysis, root-cause investigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ModelTier {
    /// Cheap, fast — keyword detection, file routing, simple classification.
    Haiku,
    /// Default — implementation, refactoring, code review.
    #[default]
    Sonnet,
    /// Expensive, deep — architecture, security analysis, root-cause investigation.
    Opus,
}

impl ModelTier {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Haiku => "haiku",
            Self::Sonnet => "sonnet",
            Self::Opus => "opus",
        }
    }

    /// Parse a tier string. Returns `None` for unknown values.
    /// Renamed from `from_str` to avoid `clippy::should_implement_trait` shadow of
    /// `std::str::FromStr::from_str` (which returns `Result`, not `Option`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "haiku" => Some(Self::Haiku),
            "sonnet" => Some(Self::Sonnet),
            "opus" => Some(Self::Opus),
            _ => None,
        }
    }
}

/// Metadata extracted from SKILL.md frontmatter for routing decisions.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SkillMetadata {
    pub name: String,
    pub context: SkillContext,
    pub agent: Option<String>,
    /// Required model tier (haiku|sonnet|opus). Defaults to Sonnet when absent.
    /// Drives cost-aware sub-agent dispatch when context: fork is set.
    pub model_tier: ModelTier,
}

struct SkillRoute {
    skill: String,
    metadata: SkillMetadata,
    ac: AhoCorasick,
}

fn skills_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|h: std::path::PathBuf| h.join(".claude/skills"))
        .unwrap_or_default()
}

fn build_route(metadata: SkillMetadata, keywords: &[String]) -> Option<SkillRoute> {
    let ac = AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(keywords)
        .ok()?;
    Some(SkillRoute {
        skill: metadata.name.clone(),
        metadata,
        ac,
    })
}

fn load_skill_data(path: &std::path::Path) -> Option<(SkillMetadata, Vec<String>)> {
    let skill_name = path.file_name()?.to_str()?.to_owned();
    let content = std::fs::read_to_string(path.join("SKILL.md")).ok()?;
    let keywords = extract_keywords(&content);
    if keywords.is_empty() {
        return None;
    }
    let mut metadata = extract_metadata(&content);
    metadata.name = skill_name;
    Some((metadata, keywords))
}

fn build_routes() -> Vec<SkillRoute> {
    let dir = skills_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| load_skill_data(&e.path()))
        .filter_map(|(meta, kws)| build_route(meta, &kws))
        .collect()
}

fn extract_triggers_array(line: &str) -> Vec<String> {
    let Some(start) = line.find('[') else {
        return Vec::new();
    };
    let Some(end) = line.find(']') else {
        return Vec::new();
    };
    let Some(arr) = line.get(start.saturating_add(1)..end) else {
        return Vec::new();
    };
    arr.split(',')
        .map(|kw| kw.trim().trim_matches('"').trim_matches('\'').to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn extract_trigger_on(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    // Look for "Trigger on:" or "Invoke on:" patterns
    let markers = ["trigger on:", "invoke on:", "triggers on:"];
    let (pos, marker_len) = markers
        .iter()
        .find_map(|m| lower.find(m).map(|p| (p, m.len())))
        .unwrap_or((0, 0));
    if marker_len == 0 {
        return Vec::new();
    }
    let Some(after) = line.get(pos.saturating_add(marker_len)..) else {
        return Vec::new();
    };
    let end = after.find('"').unwrap_or(after.len());
    let Some(slice) = after.get(..end) else {
        return Vec::new();
    };
    slice
        .split(',')
        .map(|kw| kw.trim().trim_matches('.').to_owned())
        .filter(|s| s.len() > 2)
        .collect()
}

fn extract_frontmatter(content: &str) -> Option<Vec<&str>> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first() != Some(&"---") {
        return None;
    }
    let end_idx = lines.iter().skip(1).position(|l| *l == "---")?;
    Some(lines.get(1..=end_idx)?.to_vec())
}

fn extract_metadata(content: &str) -> SkillMetadata {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return SkillMetadata::default();
    };

    let mut metadata = SkillMetadata::default();
    for line in frontmatter {
        let trimmed = line.trim();
        if trimmed.starts_with("context:") {
            let val = trimmed.strip_prefix("context:").unwrap_or("").trim();
            metadata.context = match val {
                "fork" => SkillContext::Fork,
                _ => SkillContext::Inline,
            };
        }
        if trimmed.starts_with("agent:") {
            let val = trimmed.strip_prefix("agent:").unwrap_or("").trim();
            if !val.is_empty() {
                metadata.agent = Some(val.to_owned());
            }
        }
        if trimmed.starts_with("model_tier:") {
            let val = trimmed.strip_prefix("model_tier:").unwrap_or("").trim();
            if let Some(tier) = ModelTier::parse(val) {
                metadata.model_tier = tier;
            }
        }
    }
    metadata
}

fn extract_keywords(content: &str) -> Vec<String> {
    let Some(frontmatter) = extract_frontmatter(content) else {
        return Vec::new();
    };

    let mut keywords = Vec::new();
    for line in frontmatter {
        if line.trim().starts_with("triggers:") {
            keywords.extend(extract_triggers_array(line));
        }
        if line.contains("description:") {
            keywords.extend(extract_trigger_on(line));
        }
    }
    keywords
}

fn routes() -> &'static Vec<SkillRoute> {
    static ROUTES: OnceLock<Vec<SkillRoute>> = OnceLock::new();
    ROUTES.get_or_init(build_routes)
}

/// Scan prompt text and return all skills whose keywords matched.
/// Returns deduplicated skill names in match-priority order (first match wins).
#[must_use]
pub fn skills_from_keywords(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let lower = text.to_lowercase();
    let mut matched: Vec<String> = Vec::new();
    for route in routes() {
        if route.ac.is_match(&lower) && !matched.iter().any(|s| s == &route.skill) {
            matched.push(route.skill.clone());
        }
    }
    matched
}

/// Get metadata for a skill by name.
/// Returns None if skill not found or has no keywords.
#[must_use]
pub fn get_skill_metadata(skill_name: &str) -> Option<SkillMetadata> {
    routes()
        .iter()
        .find(|r| r.skill == skill_name)
        .map(|r| r.metadata.clone())
}

/// Check if a skill should spawn as a subagent (context: fork).
#[must_use]
pub fn should_fork(skill_name: &str) -> bool {
    get_skill_metadata(skill_name).is_some_and(|m| m.context == SkillContext::Fork)
}

/// Get the agent type for a forked skill.
/// Returns None if skill is inline or has no agent specified.
#[must_use]
pub fn get_fork_agent(skill_name: &str) -> Option<String> {
    get_skill_metadata(skill_name)
        .filter(|m| m.context == SkillContext::Fork)
        .and_then(|m| m.agent)
}

/// Get the model tier for a skill.
/// Returns Sonnet (default) if skill not found or tier unset.
/// SOURCE: 42-pattern catalog §5.5 — cost-aware tier dispatch.
#[must_use]
pub fn get_model_tier(skill_name: &str) -> ModelTier {
    get_skill_metadata(skill_name)
        .map(|m| m.model_tier)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_skill(skills: &[String], name: &str) -> bool {
        skills.iter().any(|s| s == name)
    }

    #[test]
    fn should_return_empty_for_empty_input() {
        let skills = skills_from_keywords("");
        assert!(skills.is_empty());
    }

    #[test]
    fn should_detect_architecture_keywords() {
        // arch skill has triggers: ["architecture", "algorithm", "data structure", ...]
        let skills = skills_from_keywords("system design architecture scalability");
        assert!(has_skill(&skills, "arch"), "Expected arch in {skills:?}");
    }

    #[test]
    fn should_detect_data_keywords() {
        // data skill triggers on: sqlx, migration, postgresql
        let skills = skills_from_keywords("add a migration for the users table with sqlx");
        assert!(has_skill(&skills, "data"), "Expected data in {skills:?}");
    }

    #[test]
    fn should_detect_error_keywords() {
        // error skill has triggers: ["Result", "Option", "unwrap", "expect", "panic", ...]
        let skills = skills_from_keywords("fix the unwrap and expect calls with thiserror");
        assert!(has_skill(&skills, "error"), "Expected error in {skills:?}");
    }

    #[test]
    fn should_be_case_insensitive() {
        let skills = skills_from_keywords("ARCHITECTURE SYSTEM DESIGN");
        assert!(has_skill(&skills, "arch"), "Expected arch in {skills:?}");
    }

    #[test]
    fn should_load_skills_from_filesystem() {
        // Verify that routes are loaded from ~/.claude/skills/
        let loaded = routes();
        assert!(!loaded.is_empty(), "No skills loaded from filesystem");
    }

    #[test]
    fn should_extract_context_inline_by_default() {
        let content = r#"---
name: test-skill
triggers: ["test"]
---
# Test skill
"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.context, SkillContext::Inline);
        assert!(meta.agent.is_none());
    }

    #[test]
    fn should_extract_context_fork() {
        let content = r#"---
name: deep-research
context: fork
agent: Explore
triggers: ["research"]
---
# Research skill
"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.context, SkillContext::Fork);
        assert_eq!(meta.agent, Some("Explore".to_owned()));
    }

    #[test]
    fn should_extract_agent_without_fork() {
        let content = r#"---
name: review
agent: code-reviewer
triggers: ["review"]
---
# Review skill
"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.context, SkillContext::Inline);
        assert_eq!(meta.agent, Some("code-reviewer".to_owned()));
    }

    #[test]
    fn should_return_false_for_inline_skill_fork_check() {
        // Existing skills without context: fork should return false
        assert!(!should_fork("bug-bounty"));
        assert!(!should_fork("rust"));
        assert!(!should_fork("nonexistent-skill"));
    }

    #[test]
    fn should_default_model_tier_to_sonnet() {
        let content = r#"---
name: test
triggers: ["test"]
---
# Test"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.model_tier, ModelTier::Sonnet);
    }

    #[test]
    fn should_extract_model_tier_haiku() {
        let content = r#"---
name: classifier
model_tier: haiku
triggers: ["classify"]
---
# Classifier"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.model_tier, ModelTier::Haiku);
    }

    #[test]
    fn should_extract_model_tier_opus() {
        let content = r#"---
name: arch
model_tier: opus
triggers: ["architecture"]
---
# Architecture"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.model_tier, ModelTier::Opus);
    }

    #[test]
    fn should_be_case_insensitive_for_tier() {
        let content = r#"---
name: x
model_tier: OPUS
triggers: ["x"]
---"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.model_tier, ModelTier::Opus);
    }

    #[test]
    fn should_fall_back_to_default_for_unknown_tier() {
        let content = r#"---
name: x
model_tier: gpt5
triggers: ["x"]
---"#;
        let meta = extract_metadata(content);
        assert_eq!(meta.model_tier, ModelTier::Sonnet);
    }

    #[test]
    fn model_tier_round_trip() {
        for tier in [ModelTier::Haiku, ModelTier::Sonnet, ModelTier::Opus] {
            assert_eq!(ModelTier::parse(tier.as_str()), Some(tier));
        }
    }

    #[test]
    fn get_model_tier_returns_default_for_unknown_skill() {
        assert_eq!(get_model_tier("nonexistent-skill"), ModelTier::Sonnet);
    }
}
