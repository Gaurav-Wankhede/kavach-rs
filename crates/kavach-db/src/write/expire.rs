use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

const TYPED_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ExpireReport {
    pub archived_total: usize,
    pub per_table: Vec<(String, usize)>,
}

/// Archive entries past their `expires_at` timestamp across all typed memory tables.
///
/// Mirrors `kavach_db::memory::expire_stale` behavior over `SurrealDB`.
/// Returns count via .`len()` of UPDATE ... RETURN AFTER result array
/// (`SurrealDB` has no native affected-rows count; see issue #5258).
///
/// # Errors
/// Propagates `Error::Surreal` from any failed per-table UPDATE.
pub async fn expire_stale(db: &Surreal<Db>) -> Result<ExpireReport> {
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

async fn expire_table(db: &Surreal<Db>, table: &str) -> Result<usize> {
    let query = match table {
        "decision" => {
            "UPDATE decision SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "research" => {
            "UPDATE research SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "roadmap" => {
            "UPDATE roadmap SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "pattern" => {
            "UPDATE pattern SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        "app_spec" => {
            "UPDATE app_spec SET status = 'archived' WHERE expires_at != NONE AND expires_at < time::now() RETURN AFTER"
        }
        _ => return Ok(0),
    };
    let mut response = db.query(query).await?;
    let updated: Vec<serde_json::Value> = response.take(0)?;
    Ok(updated.len())
}
