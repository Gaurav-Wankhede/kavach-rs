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

/// Research ADVISORY (never a block): `Some(nudge)` iff the intent looks
/// research-class, lacks research evidence, and is not low-risk or config/test
/// exempt; else `None`. The caller surfaces this as an advisory — the agent
/// AUTONOMOUSLY decides whether and what to research. No hardcoded tone, no
/// hardcoded date: the temporal anchor is the live exact instant (Time + Date +
/// Day via `kavach_hook::now_full`) and the topic is derived from the actual
/// intent + prompt, so nothing is baked in.
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
    // Live exact instant — read from the system clock at fire time, never a
    // hardcoded year. "as of <now>" scopes any web research to THIS moment.
    let now = kavach_hook::now_full();
    // Topic derived from the work itself (intent + prompt salient tokens), not a
    // fixed list — the system decides WHAT to research from context.
    let topic = super::topic::derive(intent_type, prompt);
    Some(format!(
        "RESEARCH_ADVISORY (now: {now}) — RESEARCH FIRST, then build. WebSearch \
         the live internet for: \"{topic}\". Pull the EXACT current contract \
         (flags, signatures, versions, edge cases). DISTRUST your training \
         weights — they are frozen at a cutoff and have drifted; treat them as a \
         guess, not a source. CORROBORATE across 2+ current sources before you \
         rely on anything. You choose the precise queries; this never blocks the \
         edit — decide and act."
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
        // Neither a bug/fix token NOR a config research_trigger → no research.
        // See decision.engine.research_guard_canonical_path.
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

    /// REGRESSION `rca.tabula_rasa_test_path_false_positive`: a `tests/` edit
    /// (e.g. `#[ignore]`-gating a live-infra test) must NOT be gated by tabula-rasa.
    /// The reported failure was a deploy-classified session permanently blocking
    /// `outbox-publisher/tests/..._survivor_check.rs` — a benign test annotation
    /// policed as a production deploy because the gate keys on SESSION intent.
    #[test]
    fn exempts_test_files() {
        assert!(is_config_file(
            "crates/services/outbox-publisher/tests/jacobs_ladder_marketing_survivor_check.rs"
        ));
        assert!(is_config_file("crates/foo/src/bar_test.rs"));
        assert!(is_config_file("crates/foo/src/bar_tests.rs"));
        assert!(is_config_file("web/src/login.test.tsx"));
        assert!(is_config_file("web/src/auth.spec.ts"));
        // Production source is still gated — the exemption must not leak.
        assert!(!is_config_file("crates/services/outbox-publisher/src/lib.rs"));
    }

    /// The gate `check` returns None for a deploy-classified, high-risk session
    /// when the target is a test file — proving the SESSION-intent block no
    /// longer overrides the EDIT's actual (benign) nature.
    #[test]
    fn deploy_session_does_not_block_test_edit() {
        let session = kavach_session::SessionState::default();
        let out = super::check(
            "deploy",
            "go ahead",
            &session,
            Some("crates/x/tests/survivor_check.rs"),
        );
        assert!(out.is_none(), "test-path edit must be exempt even in a deploy session");
    }
}
