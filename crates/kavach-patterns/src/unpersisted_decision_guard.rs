//! Unpersisted-decision guard (advisory, NEVER blocking).
//!
//! `sync_to_kavach_db` says a finding lives in the DB or it is LOST. The
//! `[RCA]`/`[DESIGN]` bracket scanner only captures EXPLICITLY-marked blocks;
//! the common case is a decision SETTLED IN PROSE ("the root cause is …", "I'll
//! use X because …") with no bracket and no DB write. This detector flags that
//! gap so the stop clean-exit can nudge "you settled a decision but didn't
//! persist it" — a non-blocking advisory, consistent with the kill-blocking
//! stop-gate policy.
//!
//! SOURCE: global CLAUDE.md `sync_to_kavach_db` + `decision.stop-gate.kill-
//! blocking-keep-autocontinue` (advisory, not HALT).

/// Prose cues that a DECISION was settled this turn (a conclusion, not a
/// question or a plan). Phrased to catch the assertion, kept lowercase for a
/// single `to_lowercase` contains-scan.
const DECISION_CUES: &[&str] = &[
    "the root cause is",
    "root cause:",
    "i'll use ",
    "we'll use ",
    "decided to use",
    "the decision is",
    "i chose ",
    "we chose ",
    "going with ",
    "the fix is to",
    "the approach is",
    "settled on ",
    "the tradeoff is",
];

/// Signals the decision WAS already persisted this turn (a bracket block the
/// scanner caught, or an explicit kavach db write). If any appear, no nudge.
const PERSISTED_SIGNALS: &[&str] = &[
    "[rca]",
    "[design]",
    "[crate_decision]",
    "[arch]",
    "kavach db write",
    "category decision",
    "category research",
];

/// Detect a settled decision stated in prose that was NOT persisted this turn.
///
/// Returns `Some(advisory)` when the message asserts a decision cue AND carries
/// no persistence signal — the lost-finding signature. Returns `None` when the
/// decision was persisted, or when no decision is being asserted at all.
///
/// `wrote_decision_this_turn` is the authoritative DB-write witness from session
/// state (a `kavach db write` to a decision/research category this turn); when
/// true the nudge is always suppressed regardless of prose.
#[must_use]
pub fn detect_unpersisted_decision(msg: &str, wrote_decision_this_turn: bool) -> Option<String> {
    if wrote_decision_this_turn {
        return None;
    }
    let lower = msg.to_lowercase();
    if !DECISION_CUES.iter().any(|c| lower.contains(c)) {
        return None;
    }
    if PERSISTED_SIGNALS.iter().any(|s| lower.contains(s)) {
        return None;
    }
    Some(
        "[CAPTURE_FINDING] You settled a decision in this turn but did not persist \
         it. A finding lives in the DB or it is LOST (chat history is truncated). \
         Write it now: `kavach db write --new --project <slug> --category decision \
         --key <k> --title <t> --content <choice + one-line rationale + source>`. \
         This is an advisory, not a block."
            .to_owned(),
    )
}

#[cfg(test)]
#[path = "unpersisted_decision_guard_test.rs"]
mod tests;
