/// Model-specific configuration for token budgets.
/// SOURCE: <https://docs.rs/smart-default/0.7> — per-field defaults via derive
#[derive(Debug, Clone, smart_default::SmartDefault)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate literal DTO; non_exhaustive => E0639"
)]
pub struct ModelConfig {
    #[default(_code = "\"unknown\".to_owned()")]
    pub model_id: String,
    #[default = 200_000]
    pub context_window: i32,
    #[default = 180_000]
    pub usable_budget: i32,
}

impl ModelConfig {
    /// Detect model capabilities from model ID string.
    #[must_use]
    pub fn from_model_id(model: &str) -> Self {
        match model {
            m if m.starts_with("claude-opus-4") => Self {
                model_id: model.into(),
                context_window: 1_000_000,
                usable_budget: 950_000,
            },
            m if m.starts_with("claude-sonnet-4") => Self {
                model_id: model.into(),
                context_window: 200_000,
                usable_budget: 180_000,
            },
            m if m.starts_with("claude-haiku-4") => Self {
                model_id: model.into(),
                context_window: 200_000,
                usable_budget: 180_000,
            },
            _ => Self::default(),
        }
    }
}

/// The cheap executor tier — the fan-out target for all read/write labor.
pub const CHEAP_EXECUTOR_TIER: &str = "claude-haiku-4-5";

/// True for a frontier orchestrator tier (opus/sonnet) that should DELEGATE labor.
///
/// Haiku (the executor) and unknown ids return false — they ARE the doer, so the
/// fan-out nudge never fires on them. SOURCE: decision.harness.fanout-to-cheap-tier.
#[must_use]
pub fn is_frontier_tier(model_id: &str) -> bool {
    model_id.starts_with("claude-opus-4") || model_id.starts_with("claude-sonnet-4")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opus_model() {
        let cfg = ModelConfig::from_model_id("claude-opus-4-6");
        assert_eq!(cfg.context_window, 1_000_000);
        assert_eq!(cfg.usable_budget, 950_000);
    }

    #[test]
    fn test_sonnet_model() {
        let cfg = ModelConfig::from_model_id("claude-sonnet-4-5-20250929");
        assert_eq!(cfg.context_window, 200_000);
    }

    #[test]
    fn test_unknown_model() {
        let cfg = ModelConfig::from_model_id("gpt-4");
        assert_eq!(cfg.context_window, 200_000);
    }

    #[test]
    fn any_capable_claude_is_frontier_including_future_families() {
        // Opus/Sonnet AND newer families (Fable) are all orchestrator tiers —
        // the rule must not be a frozen opus/sonnet allowlist (that misclassified
        // claude-fable-5 as cheap). SOURCE: platform.claude.com models overview.
        assert!(is_frontier_tier("claude-opus-4-8", "claude-haiku-4-5"));
        assert!(is_frontier_tier("claude-sonnet-4-6", "claude-haiku-4-5"));
        assert!(is_frontier_tier("claude-fable-5", "claude-haiku-4-5"));
    }

    #[test]
    fn the_resolved_cheap_tier_is_never_frontier() {
        // Whatever the live cheap tier is, it is the doer, not the orchestrator.
        assert!(!is_frontier_tier("claude-haiku-4-5", "claude-haiku-4-5"));
        assert!(!is_frontier_tier("claude-haiku-9-0", "claude-haiku-9-0"));
    }

    #[test]
    fn non_claude_or_empty_is_not_frontier() {
        assert!(!is_frontier_tier("gpt-4", "claude-haiku-4-5"));
        assert!(!is_frontier_tier("", "claude-haiku-4-5"));
    }

    #[test]
    fn cheap_tier_prefers_env_over_fallback() {
        // SAFETY: test-local env mutation, single-threaded test process.
        unsafe { std::env::set_var("CLAUDE_CODE_SUBAGENT_MODEL", "claude-haiku-test-id") };
        assert_eq!(cheap_executor_tier(), "claude-haiku-test-id");
        unsafe { std::env::remove_var("CLAUDE_CODE_SUBAGENT_MODEL") };
        assert_eq!(cheap_executor_tier(), CHEAP_EXECUTOR_FALLBACK);
    }
}
