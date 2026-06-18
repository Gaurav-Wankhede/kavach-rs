use super::{SemanticDeferral, classify_semantic_deferral};

fn c(msg: &str) -> SemanticDeferral {
    classify_semantic_deferral(msg).expect("static patterns compile")
}

#[test]
fn empty_is_clear() {
    assert_eq!(c(""), SemanticDeferral::Clear);
}

#[test]
fn paraphrased_handoffs_the_regex_misses_fire() {
    for m in [
        "This is a natural stopping point; the rest is a follow-up.",
        "I've taken this as far as makes sense here.",
        "A good place to pause — someone can pick it up from here.",
        "Handing the remainder off to whoever continues.",
        "The rest should be a separate follow-up.",
        "I've pushed this as far as I can take it.",
    ] {
        assert_eq!(
            c(m),
            SemanticDeferral::ParaphrasedHandoff,
            "paraphrased handoff must fire: {m}"
        );
    }
}

#[test]
fn lexical_regex_hits_defer_to_regex_not_double_counted() {
    // These already trip detect_strategic_deferral — the backstop must yield
    // CoveredByRegex, never ParaphrasedHandoff (no double-count).
    for m in ["defer this to the backlog", "post-launch enhancement", "phase 2 work"] {
        assert_eq!(c(m), SemanticDeferral::CoveredByRegex, "regex owns: {m}");
    }
}

#[test]
fn actively_working_turns_never_fire() {
    // The PRESENT_ACTION negation must veto a handoff phrase when the turn is
    // visibly doing the work — the exact FP the deferral roster guards against.
    for m in [
        "Good stopping point reached, but let me build the next module now.",
        "The rest is a follow-up — implementing it now anyway.",
        "Handing nothing off; I'll wire the next call site this turn.",
        "As far as makes sense, so now I will continue with the next card.",
    ] {
        assert_eq!(c(m), SemanticDeferral::Clear, "active work must not fire: {m}");
    }
}

#[test]
fn ordinary_completion_prose_is_clear() {
    // A plain done-report with no handoff paraphrase must stay silent.
    for m in [
        "All three witnesses pass; the card is closed.",
        "Fixed the bug at line 42 and the build is green.",
        "The handler now returns 403 on a denied request.",
    ] {
        assert_eq!(c(m), SemanticDeferral::Clear, "neutral prose must not fire: {m}");
    }
}
