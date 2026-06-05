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
}
