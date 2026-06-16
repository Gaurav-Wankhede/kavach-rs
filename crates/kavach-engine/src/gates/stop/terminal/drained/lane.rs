//! Lane-affinity terminal verdict. A session pinned to `KAVACH_LANE=<name>`
//! only ever dispatches its own lane + the unlaned backlog (two-pass in
//! `next_open_task`). When dispatch finds nothing, THIS lane's slice + unlaned
//! are drained — foreign-lane cards may still exist, but a laned session must
//! NOT reach into them and must NOT invent PLAN phases for another lane's work.
//! The loop still re-scans this lane's DB rows and yields only to the user's
//! `Esc`; it never self-terminates.

/// The session's pinned lane, if any. Empty/unset → None → unlaned behavior.
pub(super) fn lane_env() -> Option<String> {
    std::env::var("KAVACH_LANE").ok().filter(|l| !l.is_empty())
}

/// Own lane + unlaned backlog both drained. Re-scan this lane's DB rows; never
/// cross into a foreign lane, never fabricate a plan phase. The loop yields only
/// to the user's `Esc`.
pub(super) fn lane_drained_context(lane: &str) -> String {
    kavach_hook::context_block(
        "LANE_DRAINED",
        &[
            ("lane", lane),
            (
                "why",
                "this session is pinned to the lane above (KAVACH_LANE). Its assigned \
                 lane AND the shared unlaned backlog hold no runnable card. Any \
                 remaining cards belong to a FOREIGN lane owned by another session — \
                 reaching into them would create the exact cross-session conflict \
                 lane-affinity sharding exists to prevent.",
            ),
            (
                "action",
                "Do NOT stop, and do NOT cross lanes. (1) Re-scan THIS lane's DB rows \
                 across statuses (`kavach db query --category roadmap` / `--category \
                 decision`, filtered to the lane above) for an actionable item; claim \
                 and START it. (2) Never dispatch a foreign-lane card or invent a PLAN \
                 phase. (3) When this lane holds nothing actionable, keep the loop open \
                 and yield to the user's `Esc`; new lane work is owner-assigned via \
                 `kavach db lane-set --key <k> --lane <lane>`.",
            ),
        ],
    )
}

#[cfg(test)]
#[path = "lane_test.rs"]
mod tests;
