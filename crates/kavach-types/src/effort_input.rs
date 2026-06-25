use serde::{Deserialize, Serialize};

/// Active effort level CC attaches to every hook invocation.
///
/// CC sends `{ "level": "low" | "medium" | "high" }`; gates read it to modulate
/// strictness (e.g. relax stop-gate verbosity on `low`, tighten research
/// enforcement on `high`). CC also exports `$CLAUDE_EFFORT` as a fallback.
/// SOURCE: code.claude.com/docs/en/changelog v2.1.133.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "DTO; non_exhaustive => E0639")]
pub struct EffortInput {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub level: String,
}
