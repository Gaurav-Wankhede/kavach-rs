//! Behavioral circuit breaker. Single responsibility: decide whether a
//! behavioral category should still block, or has tripped and must force-allow
//! (loop-safety) while recording a non-silent audit of the surrender.

/// `true` => block (emit `exit_stop_block`), `false` => force-allow.
///
/// FIX [`contract_violation` + `silent_failure`] — see kavach-db decision
/// `rca.stop-gate-silent-breaker-surrender`. The tripped branch used to
/// force-allow a lazy stop SILENTLY, so a determined-lazy model could repeat
/// the same anti-pattern N times and the (N+1)th stop succeeded with zero
/// audit. SOLUTION: keep loop-safety (still return `false` at the terminal)
/// but record a durable case-fact so the surrender is surfaced by the
/// forced-stop advisory. Silent => visible.
pub(crate) fn should_block_behavioral(
    session: &mut kavach_session::SessionState,
    category: &str,
) -> bool {
    if session.is_gate_tripped(category) {
        // Force-allow to preserve loop-safety — but NEVER silently. Record
        // the unresolved anti-pattern so the forced terminal names it; a
        // lazy model can no longer wait the gate out invisibly.
        let banned = format!(
            "category '{category}' tripped after {} blocks",
            session.gate_block_count(category)
        );
        let turn = session.turn_count;
        drop(kavach_session::record_mistake_surfaced(
            session,
            "behavioral_breaker_tripped",
            &banned,
            "Unresolved anti-pattern; work not complete",
            turn,
        ));
        false
    } else {
        let tripped = session.record_gate_block(category);
        !tripped // tripped => last block before force-allow
    }
}
