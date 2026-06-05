// `kavach db kanban` and `kavach db kanban-close` — SurrealDB-backed kanban view.
// SurrealDB stores entry_status directly on roadmap rows (no kanban_cards table).
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Returns true when `title` starts with `DONE` followed by any non-alphanumeric char.
pub(super) fn is_done_title(title: &str) -> bool {
    title
        .strip_prefix("DONE")
        .is_some_and(|rest| rest.starts_with(|c: char| !c.is_alphanumeric()))
}

/// Counts of non-open statuses across a roadmap.
///
#[derive(Debug, PartialEq, Eq, Default)]
pub(super) struct EmptyKanbanCounts {
    pub verified: usize,
    pub unparseable: usize,
}

const OPEN_STATUSES: &[&str] = &["todo", "in_progress", "done"];

/// Terminal status — a closed/verified roadmap row. Off the open board by
/// default; surfaced only under `--include-verified`. Canonical wire form of
/// `MemoryStatus::Verified` (kavach-types).
const VERIFIED_STATUS: &str = "verified";

/// Hunt/kanban cards are roadmap rows whose key carries this prefix.
/// Everything else is a roadmap planning item. Used to render both lenses
/// (roadmap + kanban/hunt) side-by-side in one view.
const HUNT_KEY_PREFIX: &str = "hunt.";

fn is_hunt_key(key: &str) -> bool {
    key.starts_with(HUNT_KEY_PREFIX)
}

fn is_open_status(s: &str) -> bool {
    OPEN_STATUSES.contains(&s)
}

/// Pure tally of verified/planned/unparseable counts across status strings.
pub(super) fn count_non_open<'a, I: IntoIterator<Item = &'a str>>(
    statuses: I,
) -> EmptyKanbanCounts {
    let mut out = EmptyKanbanCounts::default();
    for s in statuses {
        match s {
            VERIFIED_STATUS => out.verified = out.verified.saturating_add(1),
            "todo" | "in_progress" | "done" => {}
            other => {
                let _ = other;
                out.unparseable = out.unparseable.saturating_add(1);
            }
        }
    }
    out
}

/// Close a roadmap entry by key — sets `entry_status`='verified'.
#[expect(
    clippy::too_many_lines,
    reason = "thin CLI handler dispatching RPC and direct DB paths"
)]
pub(super) fn close(project_slug: &str, key: &str) -> i32 {
    // Try kavach-rpc daemon first; fall back to direct SurrealDB if unavailable.
    match super::rpc_client::kanban_close(project_slug, key) {
        Ok(result) if result.success => {
            let ok = format!(
                "closed [roadmap] {} (via rpc daemon)",
                result.title.unwrap_or_else(|| key.to_owned())
            );
            if let Err(io_err) = print_or_exit(&ok) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(result) => {
            let msg = format!(
                "error: {}",
                result.error.unwrap_or_else(|| "unknown".to_owned())
            );
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {
            // Daemon down → no competing RocksDB lock holder → safe to
            // fall through to the direct SurrealDB path below.
        }
        Err(e) => {
            // Daemon is UP and holds the RocksDB lock — a direct open here
            // would race it (LOCK: Resource temporarily unavailable).
            let msg = format!("rpc error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async {
        // Resilient open — closes the daemon-restart TOCTOU
        // (`rca.db-event-daemon-restart-race`): retry the lock-acquiring open
        // (bounded) instead of trusting the socket proxy; a genuine stale
        // lock still surfaces after the backoff exhausts.
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open SurrealDB: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let msg = format!("error: project not found: {project_slug}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(code) = super::validate_project_workdir(&project) {
            return code;
        }
        let Some(project_id) = project.id else {
            if let Err(io_err) = ewrite_or_exit("error: project has no id") {
                return into_exit_code(io_err);
            }
            return 1;
        };
        let entry = match kavach_surreal::get_by_key(&db, "roadmap", &project_id, key).await {
            Ok(Some(e)) => e,
            Ok(None) => {
                let msg = format!("error: no roadmap entry with key: {key}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(e) =
            kavach_surreal::update_status(&db, "roadmap", &project_id, key, "verified").await
        {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let ok = format!("closed [roadmap] {key} — {}", entry.title);
        if let Err(io_err) = print_or_exit(&ok) {
            return into_exit_code(io_err);
        }
        0
    })
}

pub(super) fn run(
    project_slug: &str,
    limit: usize,
    status_filter: Option<&str>,
    active_first: bool,
    key_filter: Option<&str>,
    include_verified: bool,
    json_output: bool,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async {
        run_async(
            project_slug,
            limit,
            status_filter,
            active_first,
            key_filter,
            include_verified,
            json_output,
        )
        .await
    })
}

async fn run_async(
    project_slug: &str,
    limit: usize,
    status_filter: Option<&str>,
    active_first: bool,
    key_filter: Option<&str>,
    include_verified: bool,
    json_output: bool,
) -> i32 {
    // Resilient open — closes the daemon-restart TOCTOU
    // (`rca.db-event-daemon-restart-race`): retry the lock-acquiring open
    // (bounded) instead of trusting the socket proxy; a genuine stale lock
    // still surfaces after the backoff exhausts.
    let db = match super::rpc_client::open_direct_resilient().await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("error: open SurrealDB: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let msg = format!("error: project not found: {project_slug}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let Some(project_id) = project.id else {
        if let Err(io_err) = ewrite_or_exit("error: project has no id") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let roadmap = match kavach_surreal::list_by_project(&db, "roadmap", &project_id).await {
        Ok(rows) => rows,
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    render_kanban(
        project_slug,
        &roadmap,
        limit,
        status_filter,
        active_first,
        key_filter,
        include_verified,
        json_output,
    )
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "thin CLI render layer — each arg is a distinct user-facing flag; a params struct would only relocate the surface, not reduce it. Line count needed for complete kanban display logic (open items, hunt partitioning, verified lens, JSON + terminal output)"
)]
fn render_kanban(
    project_slug: &str,
    roadmap: &[kavach_surreal::MemoryEntry],
    limit: usize,
    status_filter: Option<&str>,
    active_first: bool,
    key_filter: Option<&str>,
    include_verified: bool,
    json_output: bool,
) -> i32 {
    let mut open: Vec<&kavach_surreal::MemoryEntry> = roadmap
        .iter()
        .filter(|e| is_open_status(e.entry_status_str()))
        .filter(|e| status_filter.is_none_or(|s| e.entry_status_str() == s))
        .filter(|e| key_filter.is_none_or(|k| e.entry_key.contains(k)))
        .collect();

    // Optional [VERIFIED] lens: terminal rows are normally off the board.
    // `--include-verified` surfaces them so a caller can confirm a unit
    // reached `verified`, not just `done`. The `status` filter does not
    // apply (it targets open lanes); the `key` substring filter still does.
    let verified: Vec<&kavach_surreal::MemoryEntry> = if include_verified {
        roadmap
            .iter()
            .filter(|e| e.entry_status_str() == VERIFIED_STATUS)
            .filter(|e| key_filter.is_none_or(|k| e.entry_key.contains(k)))
            .collect()
    } else {
        Vec::new()
    };

    if open.is_empty() {
        let counts = count_non_open(
            roadmap
                .iter()
                .map(kavach_surreal::MemoryEntry::entry_status_str),
        );
        if json_output {
            let line = format!(
                r#"{{"items":[],"total":0,"displayed":0,"has_more":false,"verified":{},"unparseable":{}}}"#,
                counts.verified, counts.unparseable
            );
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        if counts.unparseable > 0 {
            let warn = format!(
                "warning: {} roadmap row(s) had unparseable entry_status for {project_slug} \
                 — likely legacy backlog rows not yet promoted to `todo`. \
                 Run: `kavach db query --project {project_slug} --category roadmap` to inspect, \
                 then `kavach db status-update --status todo` to migrate.",
                counts.unparseable
            );
            if let Err(io_err) = ewrite_or_exit(&warn) {
                return into_exit_code(io_err);
            }
        }
        let summary = format!(
            "kanban: no open roadmap items for {project_slug} \
             (verified={}, unparseable={}, all clear)",
            counts.verified, counts.unparseable
        );
        if let Err(io_err) = print_or_exit(&summary) {
            return into_exit_code(io_err);
        }
        if let Err(code) = render_verified_lens(&verified) {
            return code;
        }
        return 0;
    }

    if active_first {
        open.sort_by_key(|e| i32::from(e.entry_status_str() != "in_progress"));
    }

    let total = open.len();
    let displayed: Vec<_> = if limit == 0 {
        open
    } else {
        open.into_iter().take(limit).collect()
    };

    // ARCH: DualLensPartition — split open rows into roadmap vs hunt views
    // PATTERN: partition | SCOPE: render | CAP: n/a | SEARCHED: 2026-05
    // PROBLEM_CLASS: in-memory single-pass classification (no scaling concern —
    //   bounded by `limit`, default page size). TIME: O(n) | SPACE: O(n).
    // REJECTED: itertools::partition_map (groups are same type, no new dep
    //   warranted); two filter passes (2x iteration vs single partition).
    // SOURCE: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.partition
    // Both lenses at once: roadmap planning items vs kanban/hunt cards.
    // partition preserves iteration order within each group.
    let (hunt, roadmap_items): (
        Vec<&kavach_surreal::MemoryEntry>,
        Vec<&kavach_surreal::MemoryEntry>,
    ) = displayed
        .iter()
        .copied()
        .partition(|e| is_hunt_key(&e.entry_key));

    let entry_json = |e: &kavach_surreal::MemoryEntry| -> String {
        format!(
            r#"{{"key":"{}","status":"{}","title":"{}"}}"#,
            e.entry_key.replace('"', r#"\""#),
            e.entry_status_str(),
            e.title.replace('"', r#"\""#)
        )
    };

    if json_output {
        let all_json: Vec<String> = displayed.iter().map(|e| entry_json(e)).collect();
        let roadmap_json: Vec<String> = roadmap_items.iter().map(|e| entry_json(e)).collect();
        let hunt_json: Vec<String> = hunt.iter().map(|e| entry_json(e)).collect();
        // `items` retained for backward compat (harness loop parses it);
        // `roadmap` + `hunt` expose both lenses simultaneously.
        let line = format!(
            r#"{{"items":[{}],"roadmap":[{}],"hunt":[{}],"total":{},"displayed":{},"has_more":{}}}"#,
            all_json.join(","),
            roadmap_json.join(","),
            hunt_json.join(","),
            total,
            displayed.len(),
            limit > 0 && total > limit
        );
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
        return 0;
    }

    let filter_label = status_filter.map_or(String::new(), |s| format!(" status={s}"));
    if !roadmap_items.is_empty() {
        let header = format!("[ROADMAP] ({} item(s){filter_label})", roadmap_items.len());
        if let Err(io_err) = print_or_exit(&header) {
            return into_exit_code(io_err);
        }
        for entry in &roadmap_items {
            let line = format!(
                "  [{}] {} — {}",
                entry.entry_status_str(),
                entry.entry_key,
                entry.title
            );
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
    }
    if !hunt.is_empty() {
        let header = format!("[KANBAN/HUNT] ({} card(s){filter_label})", hunt.len());
        if let Err(io_err) = print_or_exit(&header) {
            return into_exit_code(io_err);
        }
        for entry in &hunt {
            let line = format!(
                "  [{}] {} — {}",
                entry.entry_status_str(),
                entry.entry_key,
                entry.title
            );
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
    }
    if limit > 0 && total > limit {
        let line = format!(
            "  ... {} more (use --limit 0 to show all)",
            total.saturating_sub(limit)
        );
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    if let Err(code) = render_verified_lens(&verified) {
        return code;
    }
    0
}

/// Render the optional `[VERIFIED]` lens — terminal rows surfaced by
/// `--include-verified`. A no-op when the slice is empty (flag off, or no
/// verified rows), so both call sites can invoke it unconditionally.
/// Returns `Err(exit_code)` on stdout failure so callers surface IO errors.
fn render_verified_lens(verified: &[&kavach_surreal::MemoryEntry]) -> Result<(), i32> {
    if verified.is_empty() {
        return Ok(());
    }
    let header = format!("[VERIFIED] ({} closed item(s))", verified.len());
    if let Err(io_err) = print_or_exit(&header) {
        return Err(into_exit_code(io_err));
    }
    for entry in verified {
        let line = format!("  [verified] {} — {}", entry.entry_key, entry.title);
        if let Err(io_err) = print_or_exit(&line) {
            return Err(into_exit_code(io_err));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_non_open_empty_input_returns_zero_for_all() {
        let counts = count_non_open(std::iter::empty::<&str>());
        assert_eq!(counts, EmptyKanbanCounts::default());
    }

    #[test]
    fn count_non_open_separates_terminal_and_unparseable_statuses() {
        // Anything outside the 4 canonical states {todo, in_progress, done,
        // verified} is unparseable — e.g. a stale string from a pre-collapse row.
        let counts = count_non_open([
            "verified", "verified", "verified", "legacy-a", "legacy-b", "garbage", "??",
        ]);
        assert_eq!(counts.verified, 3);
        assert_eq!(
            counts.unparseable, 4,
            "every non-canonical status counts as unparseable"
        );
    }

    #[test]
    fn count_non_open_ignores_open_statuses_and_counts_unparseable() {
        let counts = count_non_open(["todo", "in_progress", "done", "garbage", "stale-status"]);
        assert_eq!(
            counts.unparseable, 2,
            "two non-canonical strings are both unparseable"
        );
        assert_eq!(counts.verified, 0);
    }

    #[test]
    fn count_non_open_surfaces_corruption_signal() {
        let counts = count_non_open(["", "NULL", "Done", "DONE", "DEFERRED"]);
        assert_eq!(counts.unparseable, 5);
    }

    #[test]
    fn hunt_key_partition_predicate() {
        assert!(is_hunt_key("hunt.rpc-socket-no-auth"));
        assert!(is_hunt_key("hunt.x"));
        assert!(!is_hunt_key("P8-ws-backlog"));
        assert!(!is_hunt_key("hunting-without-dot"));
        assert!(!is_hunt_key(""));
        // Stdlib partition splits the two lenses by this exact predicate.
        let keys = ["hunt.a", "roadmap-1", "hunt.b", "P2-feature"];
        let (hunt, roadmap): (Vec<&str>, Vec<&str>) =
            keys.iter().copied().partition(|k| is_hunt_key(k));
        assert_eq!(hunt, ["hunt.a", "hunt.b"]);
        assert_eq!(roadmap, ["roadmap-1", "P2-feature"]);
    }

    #[test]
    fn done_prefix_filter_logic() {
        assert!(is_done_title("DONE: task"));
        assert!(is_done_title("DONE task"));
        assert!(is_done_title("DONE-task"));
        assert!(is_done_title("DONE_task"));
        assert!(!is_done_title("DONEtask"));
        assert!(!is_done_title("done: task"));
        assert!(!is_done_title("open task"));
        assert!(!is_done_title(""));
    }
}
