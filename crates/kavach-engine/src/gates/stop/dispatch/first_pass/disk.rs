//! Disk-pressure detection + self-heal directive for the source-down path.
//!
//! A "kanban UNREACHABLE" stop has TWO root causes that demand opposite
//! responses: a plain RPC/daemon outage (restart the daemon) versus a FULL DISK
//! (the `SurrealDB` WAL cannot append, so the DB is structurally unwritable). The
//! transcript that motivated this gate showed the model treat the second as an
//! owner-handback ("Owner — run `rm -rf ~/.cache/cargo-target`") and then spin
//! ~40 identical hold turns. That is the abolished surrender pattern wearing an
//! infra hat: the agent HOLDS the shell, so the agent frees the space itself.
//!
//! This module probes free bytes on the kavach DB volume and, when critically
//! low, returns an ACT-driven self-heal directive instead of the neutral
//! source-down text — never an `rm` command handed to the operator.
/// Below this many free bytes the `SurrealDB` WAL cannot reliably commit, so a
/// source-down is almost certainly disk-caused. 512 MiB: the transcript showed
/// the WAL append still ENOSPC at ~130 MB free, so the band is set well above
/// the observed failure point to catch the pressure BEFORE the DB wedges.
const CRITICAL_FREE_BYTES: u64 = 512 * 1024 * 1024;
/// Free bytes on the volume holding the kavach state/DB dir, or `None` if the
/// probe itself fails (treated as "not known low" — never manufacture pressure).
fn free_bytes_on_db_volume() -> Option<u64> {
    let dir = kavach_session::paths::state_dir();
    // Walk up to the nearest existing ancestor: the leaf may not exist yet, but
    // `available_space` needs a path that does. The volume is the same either way.
    let mut probe = dir.as_path();
    loop {
        if probe.exists() {
            return fs2::available_space(probe).ok();
        }
        let parent = probe.parent()?;
        probe = parent;
    }
}
/// The ACT-driven self-heal directive for a disk-caused source-down. Imperative
/// register only — the agent frees regenerable space IN-PROCESS, re-probes, and
/// completes the blocked write. It NEVER hands an `rm` to the operator and NEVER
/// holds: a full disk is a runnable task (reclaim space), not an external block.
#[must_use]
pub(super) fn self_heal_directive(free_mib: u64) -> String {
    // Tag + {free_mib} + `Esc` yield-contract frozen; the reclaim procedure is
    // research-refreshed (fail-soft to the proven literal that names `cargo clean`).
    let body = crate::gates::directive_cache::dyn_directive(
        "stop.disk-reclaim-procedure",
        "This is NOT an external block and NOT an owner task: \
         build artifacts are regenerable and YOU hold the shell, so YOU free the \
         space NOW.\n\
         DO THIS TURN (do not narrate, do not 'Hold', do not write 'Owner — run …'):\n\
         1. Reclaim regenerable space yourself — e.g. `cargo clean` in the active \
         workspace, delete `target/` of idle crates, prune `~/.cache` / `/tmp` \
         scratch, `cargo cache --autoclean` if installed. Free ≥2 GiB.\n\
         2. Re-probe: `df -h \"$HOME\"` — confirm headroom recovered.\n\
         3. Complete the BLOCKED write that wedged (the kavach `status-update` / \
         decision row), then resume dispatch from the reconciled kanban.\n\
         Handing `rm` to the operator, 'Holding', or 'no agent action can change \
         it' are FORBIDDEN here — freeing your own build scratch IS the agent \
         action. The loop yields only to the user's `Esc`.",
    );
    format!(
        "[DISK_RECLAIM — ACT, DO NOT HAND BACK] The kanban source is unreachable \
         because the DB volume is critically low ({free_mib} MiB free) — SurrealDB's \
         WAL cannot append. {body}"
    )
}
/// If the DB volume is critically low, return the self-heal directive text;
/// otherwise `None` (the caller falls back to the neutral source-down message).
#[must_use]
pub(super) fn maybe_self_heal() -> Option<String> {
    let free = free_bytes_on_db_volume()?;
    if free <= CRITICAL_FREE_BYTES {
        // `>> 20` == `/ 1 MiB`, expressed as a shift to avoid clippy integer-division.
        return Some(self_heal_directive(free >> 20));
    }
    None
}
#[cfg(test)]
#[path = "disk_test.rs"]
mod tests;
