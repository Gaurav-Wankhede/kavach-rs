use aho_corasick::AhoCorasick;
/// NLP keyword→skill routing using Aho-Corasick multi-pattern matching.
///
/// Dynamically loads keywords from ~/.claude/skills/*/SKILL.md frontmatter.
/// Single-pass O(n) scan of prompt text detects keywords that map to skills.
///
/// Supports `context: fork` and `agent: <type>` frontmatter for subagent routing.
/// Skills with `context: fork` should spawn as isolated subagents rather than
/// injecting into main context.
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
