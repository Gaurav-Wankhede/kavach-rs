// Render leaf for `kavach db kanban` — partitions open rows into roadmap vs
// hunt lenses, applies status/key/lane filters, and renders the optional
// [VERIFIED] lens. Split out of the kanban hub to keep each file ≤100 LOC.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

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

    if open.is_empty() {
        return render_empty(project_slug, roadmap, &verified, f.json);
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

    if f.json {
        return render_json(&displayed, &roadmap_items, &hunt, total, limit);
    }
    render_text(&roadmap_items, &hunt, &verified, total, limit, f.status)
}

fn render_empty(
    project_slug: &str,
    roadmap: &[kavach_surreal::MemoryEntry],
    verified: &[&kavach_surreal::MemoryEntry],
    json_output: bool,
) -> i32 {
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
        return match print_or_exit(&line) {
            Ok(()) => 0,
            Err(io_err) => into_exit_code(io_err),
        };
    }
    if counts.unparseable > 0 {
        let warn = format!(
            "warning: {} roadmap row(s) had unparseable entry_status for {project_slug} \
             — likely legacy backlog rows not yet promoted to `todo`.",
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
    if let Err(code) = render_verified_lens(verified) {
        return code;
    }
    0
}

fn entry_json(e: &kavach_surreal::MemoryEntry) -> String {
    format!(
        r#"{{"key":"{}","status":"{}","title":"{}","owner_gated":{}}}"#,
        e.entry_key.replace('"', r#"\""#),
        e.entry_status_str(),
        e.title.replace('"', r#"\""#),
        e.owner_gated.unwrap_or(false)
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

fn render_group(header: &str, rows: &[&kavach_surreal::MemoryEntry]) -> Result<(), i32> {
    if rows.is_empty() {
        return Ok(());
    }
    if let Err(io_err) = print_or_exit(header) {
        return Err(into_exit_code(io_err));
    }
    for entry in rows {
        // Surface the structured owner-gate so a reader can tell a card the loop
        // legitimately SKIPS (owner-only action pending) from runnable backlog —
        // otherwise an all-gated board reads as a stuck/non-autonomous loop.
        let gate = if entry.owner_gated.unwrap_or(false) {
            " (owner-gated)"
        } else {
            ""
        };
        let line = format!(
            "  [{}] {} — {}{gate}",
            entry.entry_status_str(),
            entry.entry_key,
            entry.title
        );
        if let Err(io_err) = print_or_exit(&line) {
            return Err(into_exit_code(io_err));
        }
    }
    Ok(())
}

fn render_text(
    roadmap_items: &[&kavach_surreal::MemoryEntry],
    hunt: &[&kavach_surreal::MemoryEntry],
    verified: &[&kavach_surreal::MemoryEntry],
    total: usize,
    limit: usize,
    status_filter: Option<&str>,
) -> i32 {
    let filter_label = status_filter.map_or(String::new(), |s| format!(" status={s}"));
    let gated = roadmap_items
        .iter()
        .chain(hunt.iter())
        .filter(|e| e.owner_gated.unwrap_or(false))
        .count();
    let gate_label = if gated > 0 {
        format!(", {gated} owner-gated")
    } else {
        String::new()
    };
    let road_header = format!(
        "[ROADMAP] ({} item(s){filter_label}{gate_label})",
        roadmap_items.len()
    );
    if let Err(code) = render_group(&road_header, roadmap_items) {
        return code;
    }
    let hunt_header = format!("[KANBAN/HUNT] ({} card(s){filter_label})", hunt.len());
    if let Err(code) = render_group(&hunt_header, hunt) {
        return code;
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
    if let Err(code) = render_verified_lens(verified) {
        return code;
    }
    0
}

/// Render the optional `[VERIFIED]` lens. A no-op when the slice is empty.
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
