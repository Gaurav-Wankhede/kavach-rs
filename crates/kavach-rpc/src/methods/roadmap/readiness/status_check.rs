/// Check if a card status is runnable (can be dispatched to an agent).
///
/// The status model is exactly four states: `todo`, `in_progress`, `done`,
/// `verified`. Only `todo` and `in_progress` are runnable; `done` awaits
/// verification and `verified` is terminal, so neither is dispatchable.
/// Dispatch paths MUST use this — a non-runnable row returned as "resume this
/// task" produces an unbreakable stop-gate loop.
///
/// Any non-canonical status string (a stale value from a pre-collapse row) is
/// fail-closed to non-runnable here — it can never be dispatched.
///
/// The string is parsed into the typed `MemoryStatus` at the boundary; an
/// unparseable value is `None` → non-runnable. The runnable SET lives on the
/// enum (`MemoryStatus::is_runnable`), not as a magic-string literal here.
#[must_use]
pub fn is_runnable_status(status: &str) -> bool {
    status
        .parse::<kavach_types::MemoryStatus>()
        .is_ok_and(kavach_types::MemoryStatus::is_runnable)
}

// PARKING ABOLISHED (operator directive 2026-06-16, reaffirmed 2026-06-17): there is
// no `is_parked` selector. A card is either RUNNABLE or DELETED — never gate-flagged
// or block-parked. The former `AGENT_BLOCKED:`/`OPERATOR-GATED:` content markers no
// longer suppress dispatch; an un-buildable card is narrowed-and-shipped or DELETED
// (`kavach db delete --category roadmap --key ...`), per global CLAUDE.md `§delete_not_park`. The dispatch
// predicate is now status + deps + umbrella only (see `lane_pick.rs`).

/// Title tokens (LOWERCASE — matched case-insensitively) that mark a card as an
/// umbrella/epic parent whose status is DERIVED from its children. Both the
/// `[UMBRELLA — …]` token form AND prose like "platform umbrella" count, so a
/// lowercase "umbrella" no longer slips into dispatch (the loop trap on
/// `platform.jacobs-ladder-marketing` / `.soundbak` / `.rainfire-missions`).
const UMBRELLA_MARKERS: [&str; 2] = ["umbrella", "epic"];

/// `true` iff the card is an umbrella/epic PARENT.
///
/// A parent has no directly agent-executable leaf work — its status is computed
/// from its children, so it must NOT be dispatched as a task (there is nothing
/// to "implement" on the parent; the children are the real units). Without this
/// the selector names the parent as `next_open_task` and the stop gate
/// re-dispatches an un-buildable epic forever. The marker lives in the TITLE
/// (`[UMBRELLA/EPIC — …]`), free-text by convention (no structural parent field
/// in schema). The ONLY non-status dispatch predicates are this and `deps_satisfied`.
#[must_use]
pub fn is_umbrella(title: &str) -> bool {
    let lowered = title.to_lowercase();
    UMBRELLA_MARKERS.iter().any(|m| lowered.contains(m))
}

/// Title phrases that mark a card as too large to build in one dispatch — it must
/// be DECOMPOSED into child roadmap rows before any leaf work is done. Unlike an
/// umbrella (whose status is purely child-derived and is never dispatched), a
/// needs-decomposition card IS still dispatched, but the stop-gate routes it to
/// an auto-decompose directive: author the children, gate the parent on them,
/// then build the children. Matching is case-insensitive on the lowered title.
const DECOMPOSITION_MARKERS: [&str; 3] = [
    "not one-card auto-build",
    "requiring decomposition",
    "needs decomposition",
];

/// `true` iff the card's title declares it too large for a one-card auto-build.
///
/// (a 127-page port, a multi-phase MAJOR unit, …). The stop-gate uses this to
/// switch the dispatch envelope from "build it" to "decompose it FIRST, then
/// build the children" (operator directive 2026-06-17: auto-decompose-then-build).
/// Without it, the selector serves the same undecomposed umbrella every turn —
/// the exact loop observed on `soundbak.dms` and `dashboard.internal-shell`.
#[must_use]
pub fn is_needs_decomposition(title: &str) -> bool {
    let lowered = title.to_lowercase();
    DECOMPOSITION_MARKERS.iter().any(|m| lowered.contains(m))
}

// PARKING FULLY ABOLISHED (operator directive 2026-06-17): the `has_inert_blocker`
// detector + `INERT_BLOCKER_LINE_PREFIXES` are REMOVED. The gate no longer
// recognizes `OPERATOR-GATED:`/`AGENT_BLOCKED:` as a signal at all — there is no
// "reconcile-first" special-casing. Dispatch is purely status + deps + umbrella;
// a card is RUNNABLE or DELETED. Decision: decision.arch.harness-degate-stale-blocker.

#[cfg(test)]
#[path = "status_check_test.rs"]
mod status_check_test;
