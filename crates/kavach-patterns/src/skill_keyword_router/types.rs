use aho_corasick::AhoCorasick;

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

/// Model tier for gates to route skills to appropriately sized models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ModelTier {
	/// Keyword detection, file routing, simple classification.
	Haiku,
	/// Default — implementation, refactoring, code review.
	#[default]
	Sonnet,
	/// Architecture, security analysis, root-cause investigation.
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

	/// Parse a tier string, returns `None` for unknown values.
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
	pub model_tier: ModelTier,
}

pub(super) struct SkillRoute {
	pub skill: String,
	pub metadata: SkillMetadata,
	pub ac: AhoCorasick,
}
