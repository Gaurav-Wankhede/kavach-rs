//! Lane-affinity terminal verdict. A session pinned to `KAVACH_LANE=<name>`
//! only ever dispatches its own lane + the unlaned backlog (two-pass in
//! `next_open_task`). When dispatch finds nothing, THIS lane's slice + unlaned
//! are drained — foreign-lane cards may still exist, but a laned session must
//! NOT reach into them and must NOT invent PLAN phases for another lane's work.
//! So it is a clean stop, distinct from the unlaned board-drained nudge.

/// The session's pinned lane, if any. Empty/unset → None → unlaned behavior.
pub(super) fn lane_env() -> Option<String> {
    std::env::var("KAVACH_LANE").ok().filter(|l| !l.is_empty())
}

/// Own lane + unlaned backlog both drained. Clean stop — never cross into a
/// foreign lane, never fabricate a plan phase for it.
pub(super) fn lane_drained_context(lane: &str) -> String {
    kavach_hook::context_block(
        "LANE_DRAINED",
        &[
            ("lane", lane),
            (
                "why",
                "this session is pinned to the lane above (KAVACH_LANE). Its assigned \
                 lane AND the shared unlaned backlog are both drained of runnable \
                 work. Any remaining cards belong to a FOREIGN lane owned by another \
                 session — reaching into them would create the exact cross-session \
                 conflict lane-affinity sharding exists to prevent.",
            ),
            (
                "action",
                "Clean stop. Do NOT dispatch a foreign-lane card, do NOT invent PLAN \
                 phases, do NOT spin. If new work for this lane is needed, the owner \
                 assigns it (`kavach db lane-set --key <k> --lane <lane>`).",
            ),
        ],
    )
}

#[cfg(test)]
#[path = "lane_test.rs"]
mod tests;
