// Render leaf for `kavach db kanban` — partitions open rows into roadmap vs
// hunt lenses, applies status/key/lane filters, and renders the optional
// [VERIFIED] lens. Split out of the kanban hub to keep each file ≤100 LOC.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};

use super::{VERIFIED_STATUS, count_non_open, is_hunt_key, is_open_status};

/// Filters used to narrow the board before rendering. Each field is a distinct
/// user-facing flag; a struct keeps the render signature within argument limits.
pub(in crate::cmd::db) struct KanbanFilters<'a> {
    pub status: Option<&'a str>,
    pub key: Option<&'a str>,
    pub lane: Option<&'a str>,
    pub active_first: bool,
    pub include_verified: bool,
    pub json: bool,
    /// `Some("dag")` = topo-tiered text DAG; `Some("mermaid")` = flowchart TD;
    /// `None` = classic flat status board. Routed in the hub before render.
    pub format: Option<&'a str>,
}

fn matches_filters(e: &kavach_surreal::MemoryEntry, f: &KanbanFilters<'_>) -> bool {
    f.status.is_none_or(|s| e.entry_status_str() == s)
        && f.key.is_none_or(|k| e.entry_key.contains(k))
        && f.lane.is_none_or(|l| e.lane.as_deref() == Some(l))
}

pub(in crate::cmd::db) fn render_kanban(
    project_slug: &str,
    roadmap: &[kavach_surreal::MemoryEntry],
    limit: usize,
    f: &KanbanFilters<'_>,
) -> i32 {
    let mut open: Vec<&kavach_surreal::MemoryEntry> = roadmap
        .iter()
        .filter(|e| is_open_status(e.entry_status_str()))
        .filter(|e| matches_filters(e, f))
        .collect();

    // Optional [VERIFIED] lens: terminal rows are normally off the board.
    // The `status` filter does not apply (it targets open lanes); the `key` and
    // `lane` filters still do.
    let verified: Vec<&kavach_surreal::MemoryEntry> = if f.include_verified {
        roadmap
            .iter()
            .filter(|e| e.entry_status_str() == VERIFIED_STATUS)
            .filter(|e| f.key.is_none_or(|k| e.entry_key.contains(k)))
            .filter(|e| f.lane.is_none_or(|l| e.lane.as_deref() == Some(l)))
            .collect()
    } else {
        Vec::new()
    };

    // JSON-only: the human-facing board is the always-on DAG (see kanban.rs).
    // This path is reached solely for `--json` (machine-parseable card list).
    let _ = (project_slug, &verified);
    if open.is_empty() {
        return render_empty_json(roadmap);
    }
    if f.active_first {
        open.sort_by_key(|e| i32::from(e.entry_status_str() != "in_progress"));
    }
    let total = open.len();
    let displayed: Vec<_> = if limit == 0 {
        open
    } else {
        open.into_iter().take(limit).collect()
    };
    let (hunt, roadmap_items): (
        Vec<&kavach_surreal::MemoryEntry>,
        Vec<&kavach_surreal::MemoryEntry>,
    ) = displayed
        .iter()
        .copied()
        .partition(|e| is_hunt_key(&e.entry_key));
    render_json(&displayed, &roadmap_items, &hunt, total, limit)
}

fn render_empty_json(roadmap: &[kavach_surreal::MemoryEntry]) -> i32 {
    let counts = count_non_open(
        roadmap
            .iter()
            .map(kavach_surreal::MemoryEntry::entry_status_str),
    );
    let line = format!(
        r#"{{"items":[],"total":0,"displayed":0,"has_more":false,"verified":{},"unparseable":{}}}"#,
        counts.verified, counts.unparseable
    );
    match print_or_exit(&line) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}

fn entry_json(e: &kavach_surreal::MemoryEntry) -> String {
    format!(
        r#"{{"key":"{}","status":"{}","title":"{}"}}"#,
        e.entry_key.replace('"', r#"\""#),
        e.entry_status_str(),
        e.title.replace('"', r#"\""#),
    )
}

fn render_json(
    displayed: &[&kavach_surreal::MemoryEntry],
    roadmap_items: &[&kavach_surreal::MemoryEntry],
    hunt: &[&kavach_surreal::MemoryEntry],
    total: usize,
    limit: usize,
) -> i32 {
    let all_json: Vec<String> = displayed.iter().map(|e| entry_json(e)).collect();
    let roadmap_json: Vec<String> = roadmap_items.iter().map(|e| entry_json(e)).collect();
    let hunt_json: Vec<String> = hunt.iter().map(|e| entry_json(e)).collect();
    let line = format!(
        r#"{{"items":[{}],"roadmap":[{}],"hunt":[{}],"total":{},"displayed":{},"has_more":{}}}"#,
        all_json.join(","),
        roadmap_json.join(","),
        hunt_json.join(","),
        total,
        displayed.len(),
        limit > 0 && total > limit
    );
    match print_or_exit(&line) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
