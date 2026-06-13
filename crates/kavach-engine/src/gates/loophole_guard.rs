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
         This change claims completion on a risk-bearing path. A loophole found is \
         a loophole you FIX THIS TURN at its root — do NOT narrate it, do NOT defer \
         it, do NOT ship a summary in place of the fix.\n\
         RUN each lens. For every lens, the verdict is exactly one of:\n\
         - FIX NOW: write the guard/check at its root this turn, then cite file:line.\n\
         - FILE: out-of-scope only -> create a roadmap card + decision row naming \
         the exact failure mode (a parked loophole is tracked, never silent).\n\
         - N/A: prove it cannot occur and cite the file:line that defends against it.\n\
         The lenses:\n\
         - concurrency: two actors at once -> TOCTOU / lost-update / double-claim. \
         CLOSE with an atomic/compare-and-swap/lock, then cite it.\n\
         - failure: process dies mid-op -> orphaned lock / half-write / leaked task. \
         CLOSE with a guard/transaction/lease-expiry, then cite it.\n\
         - malformed: null/huge/wrong-type/hostile input -> panic / injection. \
         CLOSE by validating at the edge into a typed value, then cite it.\n\
         - authz: caller without rights -> missing check / confused-deputy / IDOR. \
         CLOSE by adding the check fail-closed, then cite it.\n\
         - replay: same request twice -> non-idempotent mutation. \
         CLOSE by making it idempotent, then cite it.\n\
         - boundary: empty / max / negative / off-by-one. \
         CLOSE by handling the bound, then cite it.\n\
         Emit a `Loopholes closed:` line: each lens -> FIXED at file:line, FILED as \
         <card-key>, or N/A at file:line. A `considered`/`noted`/`should` verdict \
         without a fix or a card is NOT acceptable — close it or file it, now."
            .into(),
    )
}

/// Marker the agent emits to show it CLOSED (not merely considered) the
/// loopholes. Matched case-insensitively; its presence satisfies the Stop-gate
/// check. Imperative on purpose: `closed` means each lens was fixed at `file:line`
/// or filed as a card — a passive `considered` line no longer satisfies the gate.
const ANSWERED_MARKER: &str = "loopholes closed";

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
         [STOP] This turn shipped risk-bearing work without a `Loopholes closed:` \
         line — meaning a loophole may be live and unfixed RIGHT NOW. Do NOT stop. \
         Run the lenses on what you just shipped and CLOSE each one at its root this \
         turn (or FILE it as a card), then emit the `Loopholes closed:` line. \
         Recorded to the mistake ledger. Fixing beats documenting — fix it now."
    ))
}

#[cfg(test)]
#[path = "loophole_guard_tests.rs"]
mod tests;
