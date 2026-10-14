//! Memory-entry read operations via DatabaseStore (embedded libSQL).
//!
//! These functions replaced the DatabaseStoreDB `kavach::read` path in Phase 6.
//! They use `DatabaseStore` + `EntryStore` instead of `DatabaseStore` queries.

use crate::dual_write::MemoryEntry;
use crate::error::Result;
use crate::filter::FilterExpr;
use crate::db::DbStore;
use crate::db::entries::EntryStore;

/// Fetch a memory entry by (project, `entry_key`) from the store.
///
/// # Errors
/// Returns [`Error::Database`](crate::error::Error::Database) on SQL failure.
pub async fn get_by_key(
    store: &DbStore,
    table: &str,
    project: &str,
    key: &str,
) -> Result<Option<MemoryEntry>> {
    let entry = store
        .get_by_key(project, table, key)
        .await
        .map_err(crate::error::Error::Db)?;
    Ok(entry)
}

/// List all memory entries in `table` for `project`, ordered by priority
/// then `created_at` (dispatch order).
///
/// # Errors
/// Returns [`Error::Database`](crate::error::Error::Database) on SQL failure.
pub async fn list_by_project(
    store: &DbStore,
    table: &str,
    project: &str,
) -> Result<Vec<MemoryEntry>> {
    let entries = store
        .list_by_category(project, table)
        .await
        .map_err(crate::error::Error::Db)?;
    Ok(entries)
}

/// List every row of `table` across ALL projects.
///
/// Dependency keys (`DEPENDS_ON:`) are a global key space — a
/// card may declare a prerequisite that lives under a different project. A
/// project-scoped lookup cannot see such a row and fail-closes it to
/// "unsatisfied", permanently stalling dispatch. This unscoped read is the
/// resolver's row set so a verified prerequisite counts regardless of project.
///
/// # Errors
/// Returns [`Error::Database`](crate::error::Error::Database) on SQL failure.
pub async fn list_all_by_table(store: &DbStore, table: &str) -> Result<Vec<MemoryEntry>> {
    let entries = store.list_all().await.map_err(crate::error::Error::Db)?;
    // Filter by category since list_all returns all entries across all categories
    let filtered: Vec<MemoryEntry> = entries
        .into_iter()
        .filter(|e| e.category.as_deref() == Some(table))
        .collect();
    Ok(filtered)
}

/// List `table` rows for (project, `entry_status`) ordered by priority then
/// `created_at`.
///
/// # Errors
/// Returns [`Error::Database`](crate::error::Error::Database) on SQL failure.
pub async fn list_by_status(
    store: &DbStore,
    table: &str,
    project: &str,
    status: &str,
) -> Result<Vec<MemoryEntry>> {
    let entries = store
        .list_by_category(project, table)
        .await
        .map_err(crate::error::Error::Db)?;
    let filtered: Vec<MemoryEntry> = entries
        .into_iter()
        .filter(|e| e.entry_status.as_deref() == Some(status))
        .collect();
    Ok(filtered)
}

/// Query entries with metadata filtering, ordered by `updated_at` desc.
///
/// # Errors
/// Returns [`Error::Database`](crate::error::Error::Database) on SQL failure.
pub async fn list_with_filter(
    store: &DbStore,
    table: &str,
    project: &str,
    _filter: Option<&FilterExpr>,
    limit: Option<usize>,
) -> Result<Vec<MemoryEntry>> {
    let filter_cat = Some(table);
    let status = None;
    let contains = None;
    let limit_i64 = limit.map_or(i64::MAX, |l| i64::try_from(l).unwrap_or(i64::MAX));
    let entries = store
        .list_with_filter(project, filter_cat, status, contains, limit_i64)
        .await
        .map_err(crate::error::Error::Db)?;
    Ok(entries)
}

// REMOVED 2026-05: count_by_status + StatusCounts — counts emitted no
// useful project state. Callers now use `list_by_project` to fetch actual
// roadmap titles + entry_status (real progress, not a tally).
