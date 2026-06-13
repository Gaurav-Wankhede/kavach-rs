// hub: `kavach db kanban` / `kavach db kanban-close` — SurrealDB-backed kanban
// view. SurrealDB stores entry_status directly on roadmap rows (no kanban_cards
// table). Pure status/key helpers live here; the close path and the render path
// are leaves (close.rs, render.rs) to keep each file ≤100 LOC.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

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
        status: status_filter,
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
    // Resilient open — closes the daemon-restart TOCTOU
    // (`rca.db-event-daemon-restart-race`): retry the lock-acquiring open
    // (bounded) instead of trusting the socket proxy.
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
    // DAG awareness view: fetch the SAME dependency graph the scheduler reads and
    // project it as tiered text / mermaid, bypassing the flat status board.
    if let Some(fmt) = filters.format {
        let dag = match kavach_surreal::roadmap_dag_fetch(&db, project_slug).await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: fetch roadmap DAG: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        return dag_render::render_dag(&dag, fmt);
    }
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
    render_kanban(project_slug, &roadmap, limit, filters)
}

#[cfg(test)]
#[path = "kanban_test.rs"]
mod tests;
