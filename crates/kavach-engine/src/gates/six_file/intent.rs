// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use kavach_types::HookInput;
use std::sync::LazyLock;

use crate::error::EngineError;

// FEATURE-SURFACE intent: every verb is OBJECT-GUARDED — it must be followed by
// a new-surface noun (feature/page/endpoint/component/service/module). The prior
// regex had bare `implement ` and `decompose` alternatives with NO object, so any
// "implement the fix" / "decompose this function" misfired the gate
// (unit.gate-noise.six-file-gate-misfire). `implement` now demands the same noun
// guard `create` always had; `decompose` is dropped (it is a refactor verb, never
// new-surface). SOURCE: rca.six-file-gate-misfire (2026-06-15).
// Each build/add/implement/create verb is followed by an OBJECT GUARD: up to a
// few intervening adjective/noun words (`(?:\w+\s+){0,3}`) then a new-surface
// noun. This allows natural phrasing ("implement the notification feature",
// "build a new auth service") while still REQUIRING the surface noun — so a bare
// "implement the fix" (no surface noun within 3 words) does NOT match. The prior
// regex had bare `implement `/`decompose` with no object at all, the misfire
// root cause (unit.gate-noise.six-file-gate-misfire, rca 2026-06-15).
static INTENT_REGEX: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::RegexBuilder::new(
        r"(?:build|add|implement|create)\s+(?:a\s+|the\s+|new\s+)?(?:\w+\s+){0,3}(feature|module|service|page|endpoint|component)|draft (a )?spec|plan( |ning) (this|the|a) (build|feature|project)|new feature|next unit|what should i build|write (the|a) spec",
    )
    .case_insensitive(true)
    .build()
    .ok()
});

// NON-FEATURE intent guard: prompts whose leading intent is fix/refactor/debug/
// status/deploy/CI add NO feature surface, so the witness-chain mandate is pure
// noise that competes with the real task. If the prompt opens with one of these,
// suppress the gate even when a feature verb appears later in the sentence
// ("fix the bug, then implement the patch"). Anchored to the prompt START so a
// genuine feature request ("build a feature to fix latency") is unaffected.
static NON_FEATURE_LEAD: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::RegexBuilder::new(
        r"^\s*(please\s+)?(fix|refactor|debug|investigate|diagnose|rename|move|revert|bump|upgrade|deploy|release|ci\b|lint|format|clean ?up|reword|status|show|list|explain|why|what is|where is|how does|check|verify|review|test)\b",
    )
    .case_insensitive(true)
    .build()
    .ok()
});

#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by run_gate dispatch table: every gate handler returns Result<(), EngineError>"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let prompt = input.get_string("prompt");
    if prompt.is_empty() {
        return Ok(());
    }

    // Negative guard FIRST: a fix/refactor/status/deploy lead never adds feature
    // surface — suppress the planning mandate even if a feature verb follows.
    if NON_FEATURE_LEAD
        .as_ref()
        .is_some_and(|re| re.is_match(prompt))
    {
        return Ok(());
    }

    let Some(regex) = INTENT_REGEX.as_ref() else {
        return Ok(());
    };
    if !regex.is_match(prompt) {
        return Ok(());
    }

    emit_six_file_context();
    Ok(())
}

fn emit_six_file_context() {
    let context = r"[SIX_FILE_GATE]
Planning/feature intent detected. BEFORE Skill `writing-plans`, BEFORE any Edit/Write on new feature surface, you MUST:

  1. Invoke Skill `six-file-context` (loads kavach-db read protocol)
  2. Run the witness chain:
     kavach db get --project <slug> --category app_spec --key spec.overview
     kavach db get --project <slug> --category architecture --key-prefix arch.invariant
     kavach db get --project <slug> --category roadmap --key-prefix roadmap.unit --full
  3. If any witness returns empty, route to Agent `spec-author` (read-only) to draft missing rows; parent writes via `kavach db write`.
  4. Check `spec.scope.out.*` BEFORE adding any feature; refuse if a matching out-row exists.

Reference: ~/.claude/CLAUDE.md §15 — Six-File Context Protocol.";

    let json = format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"UserPromptSubmit","additionalContext":{context:?}}}}}"#
    );
    drop(kavach_hook::exit_prompt_context(&json));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `true` iff the prompt WOULD fire the six-file gate, factoring BOTH the
    /// non-feature lead guard and the feature-surface regex — the exact decision
    /// `run()` makes before emitting. (`run()` only returns `Ok(())` either way,
    /// so the fire-decision must be asserted on the predicates, not the result.)
    fn would_fire(prompt: &str) -> bool {
        if NON_FEATURE_LEAD.as_ref().is_some_and(|re| re.is_match(prompt)) {
            return false;
        }
        INTENT_REGEX.as_ref().is_some_and(|re| re.is_match(prompt))
    }

    // --- Positive: genuine NEW-feature-surface prompts MUST still fire ---
    #[test]
    fn fires_on_new_feature_surface() {
        assert!(would_fire("build a new feature for auth"));
        assert!(would_fire("implement the notification feature"));
        assert!(would_fire("add a new endpoint for uploads"));
        assert!(would_fire("create a component for the dashboard"));
        assert!(would_fire("BUILD a module"), "case-insensitive");
        assert!(would_fire("draft a spec for billing"));
    }

    // --- Negative (the misfire class the card names): non-feature intents ---
    #[test]
    fn silent_on_bug_fix_and_refactor() {
        // The two prompt classes the card cites verbatim.
        assert!(
            !would_fire("fix the CI refactor that broke the build"),
            "a CI-refactor request adds no feature surface"
        );
        assert!(
            !would_fire("the stop gate re-dispatches a card to a second session — bug report"),
            "a harness-bug report adds no feature surface"
        );
    }

    #[test]
    fn silent_on_verb_without_feature_object() {
        // The exact over-broad-matcher root cause: a verb with no new-surface noun.
        assert!(
            !would_fire("implement the bugfix in the parser"),
            "`implement` must demand a feature noun, not fire on any 'implement'"
        );
        assert!(
            !would_fire("decompose this 300-line function into helpers"),
            "`decompose` is a refactor verb — never new surface"
        );
    }

    #[test]
    fn silent_on_status_and_deploy_intents() {
        assert!(!would_fire("show me the current code"));
        assert!(!would_fire("what is the lease TTL?"));
        assert!(!would_fire("deploy the canary to cloudflare"));
        assert!(!would_fire("refactor the dispatch path for clarity"));
        assert!(!would_fire("bump the toolchain to 1.96"));
    }

    #[test]
    fn lead_guard_beats_a_trailing_feature_verb() {
        // A fix/refactor LEAD suppresses even when a feature verb follows later.
        assert!(
            !would_fire("fix the latency bug, then implement a feature flag"),
            "leading fix-intent must win over a trailing feature verb"
        );
    }

    // run() returns Ok(()) for all inputs (emission is a side effect); smoke it.
    #[test]
    fn run_is_ok_on_both_classes() {
        let feature =
            serde_json::from_str::<HookInput>(r#"{"prompt": "build a new feature"}"#).unwrap();
        let nonfeature =
            serde_json::from_str::<HookInput>(r#"{"prompt": "fix the build"}"#).unwrap();
        assert!(run(&feature).is_ok());
        assert!(run(&nonfeature).is_ok());
    }
}
