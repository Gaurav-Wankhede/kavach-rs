use aho_corasick::AhoCorasick;
/// NLP keyword→skill routing via Aho-Corasick from ~/.claude/skills/*/SKILL.md frontmatter.
use std::sync::OnceLock;

mod parse;
mod routes;
mod types;

#[cfg(test)]
#[path = "skill_keyword_router/tests.rs"]
mod tests;

pub use types::{ModelTier, SkillContext, SkillMetadata};
pub use routes::{
	get_model_tier, get_skill_metadata, should_fork, skills_from_keywords,
};

pub(crate) use types::SkillRoute;
pub(crate) use routes::build_routes_from;
