use crate::skill_keyword_router::parse::extract_metadata;
use crate::skill_keyword_router::routes::{build_routes_from, get_model_tier, skills_from_keywords, should_fork};
use crate::skill_keyword_router::types::{ModelTier, SkillContext, SkillRoute};

fn has_skill(skills: &[String], name: &str) -> bool {
	skills.iter().any(|s| s == name)
}

fn fixture_routes() -> (tempfile::TempDir, Vec<SkillRoute>) {
	let dir = tempfile::tempdir().expect("tempdir");
	let skills: &[(&str, &str)] = &[
		("arch", r#"["architecture", "algorithm", "data structure"]"#),
		("data", r#"["sqlx", "migration", "postgresql"]"#),
		(
			"error",
			r#"["Result", "Option", "unwrap", "expect", "panic"]"#,
		),
	];
	for (name, triggers) in skills {
		let skill_dir = dir.path().join(name);
		std::fs::create_dir_all(&skill_dir).expect("mkdir");
		let body = format!("---\nname: {name}\ntriggers: {triggers}\n---\n# {name}\n");
		std::fs::write(skill_dir.join("SKILL.md"), body).expect("write SKILL.md");
	}
	let routes = build_routes_from(dir.path());
	(dir, routes)
}

fn match_skills(routes: &[SkillRoute], text: &str) -> Vec<String> {
	if text.is_empty() {
		return Vec::new();
	}
	let lower = text.to_lowercase();
	let mut matched: Vec<String> = Vec::new();
	for route in routes {
		if route.ac.is_match(&lower) && !matched.iter().any(|s| s == &route.skill) {
			matched.push(route.skill.clone());
		}
	}
	matched
}

#[test]
fn should_return_empty_for_empty_input() {
	let skills = skills_from_keywords("");
	assert!(skills.is_empty());
}

#[test]
fn should_detect_architecture_keywords() {
	let (_dir, routes) = fixture_routes();
	let skills = match_skills(&routes, "system design architecture scalability");
	assert!(has_skill(&skills, "arch"), "Expected arch in {skills:?}");
}

#[test]
fn should_detect_data_keywords() {
	let (_dir, routes) = fixture_routes();
	let skills = match_skills(&routes, "add a migration for the users table with sqlx");
	assert!(has_skill(&skills, "data"), "Expected data in {skills:?}");
}

#[test]
fn should_detect_error_keywords() {
	let (_dir, routes) = fixture_routes();
	let skills = match_skills(&routes, "fix the unwrap and expect calls with thiserror");
	assert!(has_skill(&skills, "error"), "Expected error in {skills:?}");
}

#[test]
fn should_be_case_insensitive() {
	let (_dir, routes) = fixture_routes();
	let skills = match_skills(&routes, "ARCHITECTURE SYSTEM DESIGN");
	assert!(has_skill(&skills, "arch"), "Expected arch in {skills:?}");
}

#[test]
fn should_load_skills_from_fixture_dir() {
	let (_dir, routes) = fixture_routes();
	assert_eq!(routes.len(), 3, "fixture has 3 skills");
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
