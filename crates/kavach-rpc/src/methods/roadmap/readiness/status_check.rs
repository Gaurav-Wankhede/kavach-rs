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
