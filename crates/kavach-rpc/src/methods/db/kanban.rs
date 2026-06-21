// TIME: O(n) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
//! db.kanban RPC method — thin hub + leaf types.

use super::util::{ROADMAP_TABLE, resolve_project_id};
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::MemoryEntry;
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
        kavach_surreal::read::list_by_project(&ctx.db, ROADMAP_TABLE, &pid)
            .await
            .map_err(|e| internal(e.to_string()))?;

    let mut counts = KanbanCounts::default();
    let mut items: Vec<KanbanItem> = Vec::with_capacity(params.limit.min(entries.len()));

    for entry in &entries {
        let status_str = if entry.entry_status_str().is_empty() {
            "todo"
        } else {
            entry.entry_status_str()
        };

        let include = match (&params.status, &params.key) {
            (Some(s), _) if s != status_str => false,
            (_, Some(k)) if !entry.entry_key.contains(k) => false,
            _ => true,
        };

        if !include {
            continue;
        }

        match status_str.parse::<MemoryStatus>() {
            Ok(MemoryStatus::Todo) => counts.todo = counts.todo.saturating_add(1),
            Ok(MemoryStatus::InProgress) => {
                counts.in_progress = counts.in_progress.saturating_add(1);
            }
            Ok(MemoryStatus::Done) => counts.done = counts.done.saturating_add(1),
            Ok(MemoryStatus::Verified) => counts.verified = counts.verified.saturating_add(1),
            Ok(_) | Err(_) => {} // doctor:ok unknown/legacy status is simply not counted (no op needed)
        }

        if items.len() < params.limit {
            items.push(KanbanItem {
                key: entry.entry_key.clone(),
                title: entry.title.clone(),
                status: status_str.to_owned(),
                category: entry.category_str().to_owned(),
                content: entry.content.clone(),
            });
        }
    }

    Ok(KanbanResult { items, counts })
}
