//! Two-pass lane-affinity card selection for `next_open_task`.
//!
//! Pass 1 scans the session's OWN lane, pass 2 the unlaned (NULL) backlog; a
//! foreign lane is never inspected. With no session lane every card matches
//! pass 1, so pass 2 is a no-op and the behavior is the pre-lane single loop.

use super::super::readiness::{deps_satisfied, is_runnable_status};
use super::super::types::NextTaskResult;
use kavach_surreal::MemoryEntry;

/// `true` iff `entry`'s lane is the session's lane. With no session lane
/// (`want == None`) every card matches.
pub(super) fn lane_matches(entry: &MemoryEntry, want: Option<&str>) -> bool {
    want.is_none_or(|lane| entry.lane.as_deref() == Some(lane))
}

/// First runnable, deps-satisfied entry passing `in_lane`, in priority order.
/// `entries` is pre-sorted by priority, so the first match is the best card.
pub(super) fn pick_in_lane(
    entries: &[MemoryEntry],
    dep_pool: &[MemoryEntry],
    in_lane: impl Fn(&MemoryEntry) -> bool,
) -> Option<NextTaskResult> {
    entries
        .iter()
        .filter(|e| in_lane(e) && is_runnable_status(e.entry_status_str()))
        .find(|e| deps_satisfied(e, dep_pool))
        .map(|e| NextTaskResult {
            key: e.entry_key.clone(),
            title: e.title.clone(),
            status: e.entry_status_str().to_owned(),
        })
}

#[cfg(test)]
#[path = "lane_pick_test.rs"]
mod tests;
