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

pub(super) struct SkillRoute {
	pub skill: String,
	pub metadata: SkillMetadata,
	pub ac: AhoCorasick,
}
