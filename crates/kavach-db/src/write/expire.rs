use crate::error::Result;
use crate::db::DbStore;

const TYPED_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ExpireReport {
    pub archived_total: usize,
    pub per_table: Vec<(String, usize)>,
}

/// Archive entries past their `expires_at` timestamp across all typed memory tables.
///
/// Mirrors `kavach_db::memory::expire_stale` behavior over `DatabaseStoreDB`.
/// Counts rows via `RETURNING` (SQLite has no native affected-rows count on
/// UPDATE without RETURNING).
///
/// # Errors
/// Propagates `Error::Database` from any failed per-table UPDATE.
pub async fn expire_stale(db: &DbStore) -> Result<ExpireReport> {
    let mut report = ExpireReport::default();
    for table in TYPED_TABLES {
        let count = expire_table(db, table).await?;
        if count > 0 {
            report.per_table.push(((*table).to_owned(), count));
        }
        report.archived_total = report.archived_total.saturating_add(count);
    }
    Ok(report)
}

async fn expire_table(db: &DbStore, table: &str) -> Result<usize> {
    if !TYPED_TABLES.contains(&table) {
        return Ok(0);
    }
    let now = crate::db::now_rfc3339();
    let mut rows = db
        .exec_params(
            "UPDATE memories SET status = 'archived', updated_at = $now \
             WHERE category = $table AND expires_at IS NOT NULL AND expires_at < $now \
             RETURNING entry_key",
            libsql::params![now, table.to_owned()],
        )
        .await?;
    let mut count = 0usize;
    while rows.next().await?.is_some() {
        count = count.saturating_add(1);
    }
    Ok(count)
}
