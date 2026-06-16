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
#[must_use]
pub fn is_runnable_status(status: &str) -> bool {
    matches!(status, "todo" | "in_progress")
}

// PARKING ABOLISHED (owner directive 2026-06-16, reaffirmed 2026-06-17): there is
// no `is_parked` selector. A card is either RUNNABLE or DELETED — never gate-flagged
// or block-parked. The former `AGENT_BLOCKED:`/`OWNER-GATED:` content markers no
// longer suppress dispatch; an un-buildable card is narrowed-and-shipped or DELETED
// (`kavach db delete --category roadmap --key ...`), per global CLAUDE.md `§delete_not_park`. The dispatch
// predicate is now status + deps + umbrella only (see `lane_pick.rs`).

/// Title tokens that mark a card as an UMBRELLA/EPIC parent whose status is
/// DERIVED from its children (e.g. `[UMBRELLA/EPIC — status child-derived]`).
const UMBRELLA_MARKERS: [&str; 2] = ["UMBRELLA", "EPIC"];

/// `true` iff the card is an umbrella/epic PARENT.
///
/// A parent has no directly agent-executable leaf work — its status is computed
/// from its children, so it must NOT be dispatched as a task (there is nothing
/// to "implement" on the parent; the children are the real units). Without this
/// the selector names the parent as `next_open_task` and the stop gate
/// re-dispatches an un-buildable epic forever. The marker lives in the TITLE
/// (`[UMBRELLA/EPIC — …]`), free-text by convention (no structural parent field
/// in schema). Pairs with `is_parked`.
#[must_use]
pub fn is_umbrella(title: &str) -> bool {
    UMBRELLA_MARKERS.iter().any(|m| title.contains(m))
}
