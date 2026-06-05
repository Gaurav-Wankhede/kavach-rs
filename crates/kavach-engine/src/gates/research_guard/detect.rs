//! Research-requirement logic: bug-intent detection, research-evidence check,
//! config exemption, and the gate `check` orchestrator.
use kavach_session::SessionState;

use super::patterns::{CONFIG_EXEMPT_PATTERNS, RESEARCH_PATTERNS};

/// `true` iff the intent is a bug/fix. Explicit `debug`/`bugfix` types are a
/// fast-path; the prompt-token decision delegates to the canonical config-driven
/// `kavach_config::requires_research` so all three gate paths agree on one source
/// of truth (was: a divergent local copy → `TABULA_RASA` non-determinism).
pub(super) fn requires_research(intent_type: &str, prompt: &str) -> bool {
    if intent_type == "debug" || intent_type == "bugfix" {
        return true;
    }
    kavach_config::requires_research(prompt)
}

/// `true` iff the session shows research evidence (a `WebSearch` since intent,
/// or a research topic matching a known source).
pub(super) fn has_research(session: &SessionState) -> bool {
    if session.websearch_count_since_intent > 0 {
        return true;
    }
    session.research_topics.iter().any(|topic| {
        let lower = topic.to_lowercase();
        RESEARCH_PATTERNS.iter().any(|p| lower.contains(p))
    })
}

fn is_config_file(path: &str) -> bool {
    CONFIG_EXEMPT_PATTERNS.iter().any(|p| path.contains(p))
}

/// Gate: `Some(advisory)` iff a bug/fix intent lacks research evidence and is
/// not low-risk or config-exempt; else `None`.
pub(crate) fn check(
    intent_type: &str,
    prompt: &str,
    session: &SessionState,
    target_file: Option<&str>,
) -> Option<String> {
    if let Some(path) = target_file
        && is_config_file(path)
    {
        return None;
    }
    if !requires_research(intent_type, prompt) {
        return None;
    }
    if session.research_done && has_research(session) {
        return None;
    }
    if session.intent_risk == "low" {
        return None;
    }
    let search_year = kavach_hook::current_year();
    Some(format!(
        "RESEARCH_REQUIRED: Bug/fix intent detected. \
         WebSearch \"<topic> {search_year}\" (general search, NO site: prefix). \
         Valid sources: github.com, arxiv.org, stackoverflow.com, crates.io, docs.rs, \
         martinfowler.com, rust-lang.org, official docs. \
         DO NOT restrict to site:github.com only — use broad search."
    ))
}

#[cfg(test)]
mod tests {
    use super::{is_config_file, requires_research};

    #[test]
    fn detects_bug_fix_triggers() {
        assert!(requires_research("general", "fix the login bug"));
        assert!(requires_research("general", "resolve this issue"));
        assert!(requires_research("debug", "anything"));
        assert!(requires_research("bugfix", "anything"));
    }

    #[test]
    fn ignores_non_bug_non_trigger_prompts() {
        // Neither a bug/fix token NOR a config research_trigger
        // (implement/create/build/add/integrate/setup/configure) → no research.
        // NOTE: "create"/"add" DO now require research — they are config
        // research_triggers, and `requires_research` delegates to the canonical
        // `kavach_config` path so all three gate sites agree (the unification
        // that closed the TABULA_RASA disagreement).
        assert!(!requires_research("general", "explain how this code flows"));
        assert!(!requires_research("general", "walk me through the logic"));
    }

    #[test]
    fn exempts_config_files() {
        assert!(is_config_file("/Users/test/.claude/CLAUDE.md"));
        assert!(is_config_file("/path/to/CLAUDE.md"));
        assert!(is_config_file("/home/user/.claude/settings.json"));
        assert!(is_config_file("claude-progress.txt"));
        assert!(!is_config_file("src/main.rs"));
        assert!(!is_config_file("crates/app/lib.rs"));
    }
}
