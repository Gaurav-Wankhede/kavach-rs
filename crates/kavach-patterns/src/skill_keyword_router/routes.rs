use aho_corasick::AhoCorasick;
use std::sync::OnceLock;

use super::parse::{extract_keywords, extract_metadata};
use super::types::{ModelTier, SkillMetadata, SkillRoute};

pub(super) fn skills_dir() -> std::path::PathBuf {
	dirs::home_dir()
		.map(|h: std::path::PathBuf| h.join(".claude/skills"))
		.unwrap_or_default()
}

pub(super) fn build_route(metadata: SkillMetadata, keywords: &[String]) -> Option<SkillRoute> {
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

pub(super) fn load_skill_data(path: &std::path::Path) -> Option<(SkillMetadata, Vec<String>)> {
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

pub(super) fn build_routes() -> Vec<SkillRoute> {
	build_routes_from(&skills_dir())
}

pub(super) fn build_routes_from(dir: &std::path::Path) -> Vec<SkillRoute> {
	let Ok(entries) = std::fs::read_dir(dir) else {
		return Vec::new();
	};

	entries
		.flatten()
		.filter(|e| e.path().is_dir())
		.filter_map(|e| load_skill_data(&e.path()))
		.filter_map(|(meta, kws)| build_route(meta, &kws))
		.collect()
}

fn routes() -> &'static Vec<SkillRoute> {
	static ROUTES: OnceLock<Vec<SkillRoute>> = OnceLock::new();
	ROUTES.get_or_init(build_routes)
}

/// Scan prompt text and return all matching skills (deduplicated, first match priority).
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

/// Get metadata for a skill by name, returns None if not found.
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
	get_skill_metadata(skill_name).is_some_and(|m| m.context == super::types::SkillContext::Fork)
}

/// Get the model tier for a skill.
/// Returns Sonnet (default) if skill not found or tier unset.
#[must_use]
pub fn get_model_tier(skill_name: &str) -> ModelTier {
	get_skill_metadata(skill_name)
		.map(|m| m.model_tier)
		.unwrap_or_default()
}
