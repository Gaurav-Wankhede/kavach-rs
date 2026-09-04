use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

const STATUS_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];
const FEEDBACK_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

/// Returned-id row used by `update_status` and `kanban_close` to count
/// affected rows without triggering `SurrealDB` SDK issue #5794
/// (`Vec<serde_json::Value>` deserialization fails on records containing
/// the enum-asserted `entry_status` field).
#[derive(SurrealValue)]
pub(super) struct UpdatedIdRow {
    pub(crate) id: RecordId,
}

/// Update `entry_status` for a memory entry by typed table + `entry_key`.
///
/// # Errors
/// `Error::Migration` when `table` is not in `STATUS_TABLES`; `Error::Surreal`
/// when the UPDATE itself fails or the response shape is malformed.
pub async fn update_status(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    entry_key: &str,
    new_status: &str,
) -> Result<usize> {
    if !STATUS_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_status: unsupported table '{table}'; allowed: {STATUS_TABLES:?}"
        )));
    }
    let query = format!(
        "UPDATE {table} SET entry_status = $status, updated_at = time::now() \
         WHERE project = $project AND entry_key = $key RETURN id"
    );
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("status", new_status.to_owned()))
        .await?;
    let updated: Vec<UpdatedIdRow> = response.take(0)?;
    let count = updated.len();
    if let Some(first) = updated.first() {
        let _ = &first.id.table;
    }
    Ok(count)
}

/// Atomically transition `entry_status` only when the row's CURRENT status
/// equals `expected`. Returns the number of rows actually transitioned (0 or 1).
///
/// This is the single-statement compare-and-set that closes the claim-card
/// TOCTOU race: the `WHERE entry_status = $expected` predicate is evaluated and
/// the write applied inside ONE `UPDATE`, so two sessions racing to claim the
/// same `todo` card cannot both succeed — `SurrealDB` evaluates the predicate
/// against the row state at write time, and only the first writer matches. A
/// returned count of 0 means "another session already moved it" (lost the
/// race), NOT an error. Prefer this over the read-then-`update_status` pattern
/// for any contended transition.
///
/// # Errors
/// `Error::Migration` when `table` is not in `STATUS_TABLES`; `Error::Surreal`
/// when the UPDATE itself fails or the response shape is malformed.
pub async fn update_status_cas(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    entry_key: &str,
    expected: &str,
    new_status: &str,
) -> Result<usize> {
    if !STATUS_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_status_cas: unsupported table '{table}'; allowed: {STATUS_TABLES:?}"
        )));
    }
    let query = format!(
        "UPDATE {table} SET entry_status = $status, updated_at = time::now() \
         WHERE project = $project AND entry_key = $key AND entry_status = $expected \
         RETURN id"
    );
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("expected", expected.to_owned()))
        .bind(("status", new_status.to_owned()))
        .await?;
    let updated: Vec<UpdatedIdRow> = response.take(0)?;
    let count = updated.len();
    if let Some(first) = updated.first() {
        let _ = &first.id.table;
    }
    Ok(count)
}

/// Update the `feedback` field for a memory entry.
///
/// # Errors
/// `Error::Migration` when `table` is not in `FEEDBACK_TABLES`; `Error::Surreal`
/// when the UPDATE fails.
pub async fn update_feedback(
    db: &Surreal<Db>,
    table: &str,
    project_id: &RecordId,
    entry_key: &str,
    feedback: &str,
) -> Result<usize> {
    if !FEEDBACK_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_feedback: unsupported table '{table}'"
        )));
    }
    let query = format!(
        "UPDATE {table} SET feedback = $feedback, updated_at = time::now() \
         WHERE project = $project AND entry_key = $key RETURN id"
    );
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("feedback", feedback.to_owned()))
        .await?;
    let updated: Vec<UpdatedIdRow> = response.take(0)?;
    Ok(updated.len())
}
