// TIME: O(n) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
//! db.kanban RPC method — thin hub + leaf types.

use super::util::{ROADMAP_TABLE, resolve_project_id};
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_db::MemoryEntry;
use kavach_types::MemoryStatus;

mod types;

pub use types::{KanbanCounts, KanbanItem, KanbanParams, KanbanResult};

/// Retrieve a kanban view of entries.
///
/// # Errors
///
/// Returns an error if the project is not found or if the database read fails.
pub async fn kanban(
    ctx: &AppState,
    params: KanbanParams,
) -> Result<KanbanResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let entries: Vec<MemoryEntry> =
        kavach_db::read::list_by_project(&ctx.db, ROADMAP_TABLE, &pid)
            .await
            .map_err(|e| internal(e.to_string()))?;

    let mut counts = KanbanCounts::default();
    let mut items: Vec<KanbanItem> = Vec::with_capacity(params.limit.min(entries.len()));

    // Lexed once: the status filter is a token, and every row matches on its
    // typed lane — no per-row `== "…"` string scans.
    let want = kavach_db::StatusPred::from_opt(params.status.as_deref());

    for entry in &entries {
        let typed = entry.effective_status_typed();
        if !want.matches_typed(typed) || !key_matches(entry, params.key.as_deref()) {
            continue;
        }

        match typed {
            Some(MemoryStatus::Todo) => counts.todo = counts.todo.saturating_add(1),
            Some(MemoryStatus::InProgress) => {
                counts.in_progress = counts.in_progress.saturating_add(1);
            }
            Some(MemoryStatus::Done) => counts.done = counts.done.saturating_add(1),
            Some(MemoryStatus::Verified) => counts.verified = counts.verified.saturating_add(1),
            // doctor:ok unknown/legacy status is simply not counted (no op needed)
            Some(_) | None => {}
        }

        if items.len() < params.limit {
            items.push(KanbanItem {
                key: entry.entry_key.clone(),
                title: entry.title.clone(),
                status: entry.effective_status().to_owned(),
                category: entry.category_str().to_owned(),
                content: entry.content.clone(),
            });
        }
    }

    Ok(KanbanResult { items, counts })
}

/// Key substring gate (unchanged string path: keys are open-ended identifiers,
/// not a keyword vocabulary, so there is nothing to intern).
fn key_matches(entry: &MemoryEntry, key: Option<&str>) -> bool {
    key.is_none_or(|k| entry.entry_key.contains(k))
}
