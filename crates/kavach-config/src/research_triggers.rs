//! Canonical bug/fix research-trigger tokens — the single source of truth for
//! "does this prompt describe a bug/fix that demands research before code?".
//!
//! WHY this exists: three sites used to decide `requires_research` independently
//! (`blocklist::requires_research` config-driven, `research_guard::detect` hardcoded,
//! `intent_tree` per-leaf constant) and DISAGREED on the same prompt — the
//! `TABULA_RASA` non-determinism. They now all consult this list, so the config
//! file can only *extend* the floor, never silently drop below it (fail-closed).
//! SOURCE: <https://martinfowler.com/bliki/SingleSourceOfTruth.html>
/// Canonical bug/fix research-trigger tokens.
///
/// Research is required regardless of the user-tunable `research_triggers` in
/// `~/.claude/gates/config.json`. This is the non-negotiable floor: even an
/// empty config must still gate bug work.
pub const BUG_FIX_TRIGGERS: &[&str] = &[
    "fix",
    "bug",
    "issue",
    "error",
    "broken",
    "crash",
    "fail",
    "regression",
    "patch",
    "hotfix",
    "resolve",
    "debug",
];
/// `true` iff `lower` (already lowercased) contains any canonical bug/fix token.
#[must_use]
pub fn has_bug_fix_trigger(lower: &str) -> bool {
    BUG_FIX_TRIGGERS.iter().any(|t| lower.contains(t))
}
#[cfg(test)]
#[path = "research_triggers_test.rs"]
#[cfg(test)]
#[path = "research_triggers_test.rs"]
mod tests;
