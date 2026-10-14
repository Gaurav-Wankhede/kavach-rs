// hub: `kavach db kanban` / `kavach db kanban-close` — Database-backed kanban
// view. Database stores entry_status directly on roadmap rows (no kanban_cards
// table). Pure status/key helpers live here; the close path and the render path
// are leaves (close.rs, render.rs) to keep each file ≤100 LOC.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};
use kavach_db::db::entries::EntryStore;
use kavach_types::MemoryStatus;
mod close;
mod dag_render;
mod render;
pub(super) use close::close;
use render::{KanbanFilters, render_kanban};
/// Returns true when `title` starts with `DONE` followed by any non-alphanumeric char.
pub(super) fn is_done_title(title: &str) -> bool {
    title
        .strip_prefix("DONE")
        .is_some_and(|rest| rest.starts_with(|c: char| !c.is_alphanumeric()))
}
/// Counts of non-open statuses across a roadmap.
#[derive(Debug, PartialEq, Eq, Default)]
pub(super) struct EmptyKanbanCounts {
    pub verified: usize,
    pub unparseable: usize,
}
const OPEN_STATUSES: &[&str] = &["todo", "in_progress", "done"];
/// Terminal status — a closed/verified roadmap row. Off the open board by
/// default; surfaced only under `--include-verified`.
pub(super) const VERIFIED_STATUS: &str = "verified";
/// Hunt/kanban cards are roadmap rows whose key carries this prefix.
const HUNT_KEY_PREFIX: &str = "hunt.";
pub(super) fn is_hunt_key(key: &str) -> bool {
    key.starts_with(HUNT_KEY_PREFIX)
}
pub(super) fn is_open_status(s: &str) -> bool {
    OPEN_STATUSES.contains(&s)
}
/// Pure tally of verified/unparseable counts across status strings.
pub(super) fn count_non_open<'a, I: IntoIterator<Item = &'a str>>(
    statuses: I,
) -> EmptyKanbanCounts {
    let mut out = EmptyKanbanCounts::default();
    for s in statuses {
        match s {
            VERIFIED_STATUS => out.verified = out.verified.saturating_add(1),
            "todo" | "in_progress" | "done" => {}
            _ => out.unparseable = out.unparseable.saturating_add(1),
        }
    }
    out
}
#[expect(
    clippy::too_many_arguments,
    reason = "thin CLI handler — each arg is a distinct user-facing flag"
)]
pub(super) fn run(
    project_slug: &str,
    limit: usize,
    status_filter: Option<&str>,
    active_first: bool,
    key_filter: Option<&str>,
    lane_filter: Option<&str>,
    include_verified: bool,
    json_output: bool,
    format: Option<&str>,
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
    let filters = KanbanFilters {
        // Lexed once here: every row match below is a discriminant compare.
        status: kavach_db::StatusPred::from_opt(status_filter),
        key: key_filter,
        lane: lane_filter,
        active_first,
        include_verified,
        json: json_output,
        format,
    };
    runtime.block_on(run_async(project_slug, limit, &filters))
}
async fn run_async(project_slug: &str, limit: usize, filters: &KanbanFilters<'_>) -> i32 {
    // Phase 6: direct open via DatabaseStore (libSQL file — no lock contention).
    let store = match super::rpc_client::open_turso_resilient().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("error: open kavach.db: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let roadmap = match store.list_by_category(project_slug, "roadmap").await {
        Ok(rows) => rows,
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    // Non-roadmap `entry_key`s, so a dep on a DECISION or RESEARCH row resolves
    // instead of rendering as a missing prerequisite. Without this a genuine
    // cross-category edge (`model-shift-router-advisory`, a verified decision)
    // showed as an `(unresolved)` phantom and looked deletable.
    let other_keys: Vec<String> = match store.list_by_project(project_slug).await {
        Ok(all) => all
            .into_iter()
            .filter(|e| e.category.as_deref() != Some("roadmap"))
            .map(|e| e.entry_key)
            .collect(),
        Err(_) => Vec::new(),
    };
    // The DAG is the ALWAYS-ON default awareness surface: every `kavach db kanban`
    // read projects the dependency graph (tiered text by default, or mermaid via
    // `--format mermaid`) so the agent always sees the exact task graph + which
    // cards are done/ready/blocked — never a dependency-blind flat list. Built
    // from the SAME roadmap rows + declared DEPENDS_ON: deps the
    // scheduler uses, so the view and the dispatch order agree by construction.
    // `--json` keeps the machine-parseable card list for tooling that needs it.
    if filters.json {
        return render_kanban(project_slug, &roadmap, limit, filters);
    }
    // K-5: lane/status/key filters apply BEFORE the DAG render — previously the
    // non-`--json` path ignored them and always rendered the full graph, so
    // `--status done` showed the missing-refs view instead of done rows.
    let has_filter = filters.status != kavach_db::StatusPred::Any
        || filters.key.is_some()
        || filters.lane.is_some();
    if !has_filter {
        let fmt = filters.format.unwrap_or("dag");
        return dag_render::render_dag_from_rows(&roadmap, fmt, &other_keys);
    }
    let filtered: Vec<kavach_db::MemoryEntry> = roadmap
        .iter()
        .filter(|e| {
            filters.status.matches_typed(e.effective_status_typed())
                && filters.key.is_none_or(|k| e.entry_key.contains(k))
                && filters.lane.is_none_or(|l| e.lane.as_deref() == Some(l))
        })
        .cloned()
        .collect();
    // An explicit `--status done|verified` asks for closed rows: show them as
    // tiers instead of hiding them as "already closed".
    let show_closed = matches!(
        filters.status,
        kavach_db::StatusPred::Is(MemoryStatus::Done | MemoryStatus::Verified)
    );
    let fmt = filters.format.unwrap_or("dag");
    dag_render::render_dag_from_filtered(&filtered, fmt, show_closed, limit, &other_keys)
}
#[cfg(test)]
#[path = "kanban_test.rs"]
mod tests;
