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

/// The content markers that HONESTLY park a card (owner-only / agent-can't-build).
/// Mirrors `kavach_engine` `stop_dispatch::card::PARK_MARKERS` — kept in sync by
/// the shared semantics, not a shared import (the engine crate is downstream).
const PARK_MARKERS: [&str; 2] = ["AGENT_BLOCKED:", "OWNER-GATED:"];

/// `true` iff the card is honestly parked.
///
/// A parked card carries an `AGENT_BLOCKED:` or `OWNER-GATED:` line in its
/// content. It is owner-only (no agent-executable work) and MUST be excluded
/// from dispatch — otherwise the selector keeps naming it as `next_open_task`
/// and the stop gate re-dispatches it forever (the `owner_gated` schema field
/// was removed 2026-06-16; this content marker is its replacement). Without this
/// filter the dispatch predicate (status + deps) alone re-selects a parked card
/// every iteration. SOURCE: `card.rs:110` reads `entry.content`.
#[must_use]
pub fn is_parked(content: &str) -> bool {
    PARK_MARKERS.iter().any(|m| content.contains(m))
}

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
