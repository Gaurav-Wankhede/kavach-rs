//! Semantic deferral backstop — the LLM-judge pass that catches PARAPHRASED
//! handoffs the literal `deferral_pattern()` regex misses.
//!
//! `deferral_pattern()` (presets.rs) enumerates exact forbidden phrases; a model
//! that paraphrases ("I'll leave the rest to you", "feel free to take it from
//! here") evades it. This backstop classifies deferral INTENT semantically: a
//! Stop message that both (a) hands the work away with a handoff verb AND (b)
//! aims it at a second-person actor is a deferral, however it is worded.
//!
//! Conservative by construction (mirrors `ai_verdict.rs`): a positive needs BOTH
//! signal classes to co-occur, so an ordinary completion summary that merely
//! contains "you" (e.g. "this lets you read the trail") does NOT trip it. The
//! judge is a pure `fn(&str) -> bool` — deterministic, no network call — so
//! `reward.rs` stays same-input-same-output (INV-2). It runs ONLY when the cheap
//! regex did not already fire (the caller gates it), so the regex remains the
//! first pass and the judge is the ambiguous-Stop backstop.
//!
//! SOURCE: card roadmap.unit.infra.kavach-harness.semantic-deferral-detector;
//! global CLAUDE.md forbidden-handoff-phrases law; RLAIF judge idiom (`ai_verdict.rs`).
/// Handoff verbs/phrases that signal the assistant is RELINQUISHING the work
/// rather than doing it. Paraphrase-robust stems the literal regex lacks.
const HANDOFF_SIGNALS: [&str; 10] = [
    "leave that to you",
    "leave the rest to you",
    "leave it to you",
    "take it from here",
    "take over from here",
    "up to you to",
    "feel free to continue",
    "feel free to take",
    "when you're ready",
    "whenever you're ready",
];
/// Second-person-actor cues that confirm the relinquished work is aimed at the
/// USER. Required to co-occur with a handoff signal so neutral "you" prose in a
/// genuine completion summary is not misread as a deferral (fail-safe to false).
const ACTOR_SIGNALS: [&str; 6] = [
    "you can ",
    "you could ",
    "you may ",
    "you might want",
    "you'll want",
    "if you'd like",
];
/// Classify a Stop `final_message` as a semantic deferral/false-blocker handoff.
///
/// Returns `true` ONLY when a handoff signal AND a second-person-actor cue both
/// appear — the conservative AND that keeps ordinary completion summaries from
/// tripping. Case-insensitive on the trimmed message. Intended as the backstop
/// the caller invokes when the literal `deferral_pattern()` regex did not match.
#[must_use]
pub fn is_semantic_deferral(message: &str) -> bool {
    let lowered = message.trim().to_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let has_handoff = HANDOFF_SIGNALS.iter().any(|s| lowered.contains(s));
    let has_actor = ACTOR_SIGNALS.iter().any(|s| lowered.contains(s));
    has_handoff && has_actor
}
#[cfg(test)]
#[path = "semantic_deferral_test.rs"]
#[cfg(test)]
#[path = "semantic_deferral_test.rs"]
mod tests;
