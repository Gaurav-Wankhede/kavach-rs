use crate::error::Result;
use crate::db::DbStore;

const STATUS_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];
const FEEDBACK_TABLES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

/// Update `entry_status` for a memory entry by category + `entry_key`.
///
/// # Errors
/// `Error::Migration` when `table` is not in `STATUS_TABLES`; `Error::Database`
/// when the UPDATE itself fails.
pub async fn update_status(
    db: &DbStore,
    table: &str,
    project_id: &str,
    entry_key: &str,
    new_status: &str,
) -> Result<usize> {
    if !STATUS_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_status: unsupported table '{table}'; allowed: {STATUS_TABLES:?}"
        )));
    }
    let now = crate::db::now_rfc3339();
    let mut response = db
        .exec_params(
            "UPDATE memories SET entry_status = $status, updated_at = $now \
             WHERE project = $project AND category = $table AND entry_key = $key \
             RETURNING entry_key",
            libsql::params![
                new_status.to_owned(),
                now,
                project_id.to_owned(),
                table.to_owned(),
                entry_key.to_owned()
            ],
        )
        .await?;
    let mut count = 0usize;
    while response.next().await?.is_some() {
        count = count.saturating_add(1);
    }
    Ok(count)
}

/// Atomically transition `entry_status` only when the row's CURRENT status
/// equals `expected`. Returns the number of rows actually transitioned (0 or 1).
///
/// This is the single-statement compare-and-set that closes the claim-card
/// TOCTOU race: the `WHERE entry_status = $expected` predicate is evaluated and
/// the write applied inside ONE `UPDATE`, so two sessions racing to claim the
/// same `todo` card cannot both succeed — SQLite evaluates the predicate
/// against the row state at write time, and only the first writer matches. A
/// returned count of 0 means "another session already moved it" (lost the
/// race), NOT an error. Prefer this over the read-then-`update_status` pattern
/// for any contended transition.
///
/// # Errors
/// `Error::Migration` when `table` is not in `STATUS_TABLES`; `Error::Database`
/// when the UPDATE itself fails.
pub async fn update_status_cas(
    db: &DbStore,
    table: &str,
    project_id: &str,
    entry_key: &str,
    expected: &str,
    new_status: &str,
) -> Result<usize> {
    if !STATUS_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_status_cas: unsupported table '{table}'; allowed: {STATUS_TABLES:?}"
        )));
    }
    let now = crate::db::now_rfc3339();
    let mut response = db
        .exec_params(
            "UPDATE memories SET entry_status = $status, updated_at = $now \
             WHERE project = $project AND category = $table AND entry_key = $key \
               AND entry_status = $expected \
             RETURNING entry_key",
            libsql::params![
                new_status.to_owned(),
                now,
                project_id.to_owned(),
                table.to_owned(),
                entry_key.to_owned(),
                expected.to_owned()
            ],
        )
        .await?;
    let mut count = 0usize;
    while response.next().await?.is_some() {
        count = count.saturating_add(1);
    }
    Ok(count)
}

/// Update the `feedback` field for a memory entry.
///
/// # Errors
/// `Error::Migration` when `table` is not in `FEEDBACK_TABLES`; `Error::Database`
/// when the UPDATE fails.
pub async fn update_feedback(
    db: &DbStore,
    table: &str,
    project_id: &str,
    entry_key: &str,
    feedback: &str,
) -> Result<usize> {
    if !FEEDBACK_TABLES.contains(&table) {
        return Err(crate::error::Error::Migration(format!(
            "update_feedback: unsupported table '{table}'"
        )));
    }
    let now = crate::db::now_rfc3339();
    let mut response = db
        .exec_params(
            "UPDATE memories SET feedback = $feedback, updated_at = $now \
             WHERE project = $project AND category = $table AND entry_key = $key \
             RETURNING entry_key",
            libsql::params![
                feedback.to_owned(),
                now,
                project_id.to_owned(),
                table.to_owned(),
                entry_key.to_owned()
            ],
        )
        .await?;
    let mut count = 0usize;
    while response.next().await?.is_some() {
        count = count.saturating_add(1);
    }
    Ok(count)
}
