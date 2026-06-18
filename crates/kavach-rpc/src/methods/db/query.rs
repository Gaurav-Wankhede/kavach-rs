// TIME: O(n) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
//! db.query RPC method — thin hub + types.

use super::util::{ROADMAP_TABLE, or_str, resolve_project_id};
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::MemoryEntry;

mod types;

pub use types::{QueryEntry, QueryParams, QueryResult};

const STATUS_VERIFIED: &str = "verified";
const STATUS_TODO: &str = "todo";
const ACCESS_COUNT_DEFAULT: i64 = 0;

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the underlying `SurrealDB` query fails.
pub async fn query(ctx: &AppState, params: QueryParams) -> Result<QueryResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let table = or_str(params.category.as_deref(), ROADMAP_TABLE);
    let entries: Vec<MemoryEntry> = kavach_surreal::read::list_by_project(&ctx.db, table, &pid)
        .await
        .map_err(|e| internal(e.to_string()))?;

    let all_entries = params.all.unwrap_or_default();

    let entries: Vec<QueryEntry> = entries
        .into_iter()
        .filter(|e| {
            if all_entries {
                true
            } else {
                let status = if e.entry_status_str().is_empty() {
                    STATUS_TODO
                } else {
                    e.entry_status_str()
                };
                status != STATUS_VERIFIED
            }
        })
        .map(|e| {
            let access_count = e.access_count.unwrap_or(ACCESS_COUNT_DEFAULT);
            QueryEntry {
                key: e.entry_key.clone(),
                title: e.title.clone(),
                category: e.category_str().to_owned(),
                status: if e.entry_status_str().is_empty() {
                    STATUS_TODO.to_owned()
                } else {
                    e.entry_status_str().to_owned()
                },
                content: Some(e.content.clone()),
                access_count,
            }
        })
        .collect();

    Ok(QueryResult { entries })
}
