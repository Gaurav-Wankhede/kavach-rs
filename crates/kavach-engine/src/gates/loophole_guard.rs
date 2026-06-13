//! Loophole self-interrogation prompt — the enforcement teeth behind the
//! `loophole_self_interrogation` directive in the global CLAUDE.md / Cursor rules.
//!
//! A loophole is a defect the happy path never exercises, so a clean build and a
//! green test suite do NOT prove its absence — only an adversarial question does.
//! This guard injects that question at the exact moment it matters: when written
//! content claims completion AND touches a risk-bearing path (auth / lease / lock
//! / money / persistence / concurrency / state transition). It is `P1Advisory`, NOT
//! a block — per the engine severity policy, a "did you think about loopholes?"
//! hard-block would false-positive on every trivial card. The model is reminded,
//! scoped to where it counts; it is never stopped.

/// Completion-claim phrases — the trigger half. Mirrors `completion_guard` but
/// kept local so the two guards stay independently tunable.
const DONE_PHRASES: &[&str] = &[
    "done",
    "complete",
    "finished",
    "shipped",
    "implemented",
    "fixed",
    "verified",
    "works now",
    "ready",
];

/// Risk-bearing path markers — the scope half. Only content touching one of
/// these warrants the adversarial prompt; a docs/rename/format change does not.
const RISK_MARKERS: &[&str] = &[
    "auth",
    "lease",
    "lock",
    "mutex",
    "rwlock",
    "token",
    "session",
    "password",
    "secret",
    "payment",
    "balance",
    "transfer",
    "transaction",
    "persist",
    "commit",
    "concurren",
    "atomic",
    "race",
    "status",
    "state_transition",
    "claim",
    "acquire",
    "permission",
    "authorize",
];

/// Return the loophole self-interrogation advisory when `content` BOTH claims
/// completion AND touches a risk-bearing path. `None` otherwise — the common
/// case, so trivial work is never nagged.
pub(crate) fn check_loophole_interrogation(content: &str) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    let lower = content.to_lowercase();
    let claims_done = DONE_PHRASES.iter().any(|p| lower.contains(p));
    if !claims_done {
        return None;
    }
    let touches_risk = RISK_MARKERS.iter().any(|m| lower.contains(m));
    if !touches_risk {
        return None;
    }
    Some(
        "[LOOPHOLE_CHECK]\n\
         This change claims completion on a risk-bearing path. Before declaring \
         done, self-ask: \"What is the loophole here? How would a hostile / \
         concurrent / malformed / crashed actor break this?\"\n\
         Run the lenses and answer EACH with evidence:\n\
         - concurrency: two actors at once -> TOCTOU / lost-update / double-claim?\n\
         - failure: process dies mid-op -> orphaned lock / half-write / leaked task?\n\
         - malformed: null/huge/wrong-type/hostile input -> panic / injection?\n\
         - authz: caller without rights -> missing check / confused-deputy / IDOR?\n\
         - replay: same request twice -> non-idempotent mutation?\n\
         - boundary: empty / max / negative / off-by-one?\n\
         Emit a `Loopholes considered:` line: each lens -> closed at file:line, \
         recorded as a card, or proven N/A. Silence is NOT proof of safety."
            .into(),
    )
}

/// Marker the agent emits to show it ran the self-interrogation. Matched
/// case-insensitively; its presence is what satisfies the Stop-gate check.
const ANSWERED_MARKER: &str = "loopholes considered";

/// Stop-gate variant: given the final assistant `message` of a turn, return the
/// loophole advisory when the turn claimed completion on a risk-bearing path but
/// emitted NO `Loopholes considered:` line. `None` otherwise — so a turn that
/// either did no risk work, made no completion claim, OR already answered the
/// self-interrogation exits clean.
///
/// This is the Stop-gate's teeth for the loophole directive: it does NOT halt
/// (per the "kill blocking, keep auto-continue" policy) — the caller appends the
/// result as a clean-exit ride-along advisory AND records a mistake-ledger row,
/// feeding the learning loop so the omission is seen over time.
pub(crate) fn check_stop_interrogation(message: &str) -> Option<String> {
    let base = check_loophole_interrogation(message)?;
    // Already answered -> satisfied, no nudge.
    if message.to_lowercase().contains(ANSWERED_MARKER) {
        return None;
    }
    Some(format!(
        "{base}\n\
         [STOP] This turn closed risk-bearing work without a `Loopholes \
         considered:` line. Recorded to the mistake ledger so the system stays \
         aware. Next risk-bearing turn: answer the lenses BEFORE the stop."
    ))
}

#[cfg(test)]
#[path = "loophole_guard_tests.rs"]
mod tests;
