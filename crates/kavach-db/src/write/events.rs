use crate::error::Result;
use crate::db::DbStore;

/// Delete events older than `days` days. Returns count deleted.
///
/// # Errors
/// Propagates `Error::Database` from the DELETE query.
pub async fn rotate_events(db: &DbStore, days: i64) -> Result<usize> {
    let floor_secs = days.saturating_mul(86_400);
    let floor_ts = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::seconds(floor_secs))
        .unwrap_or_else(chrono::Utc::now);
    let floor_str = floor_ts.to_rfc3339();
    let mut rows = db
        .exec_params(
            "DELETE FROM events WHERE created_at < $floor RETURNING id",
            libsql::params![floor_str],
        )
        .await?;
    let mut deleted = 0usize;
    while rows.next().await?.is_some() {
        deleted = deleted.saturating_add(1);
    }
    Ok(deleted)
}

/// Append an event row.
///
/// # Errors
/// Propagates `Error::Database` from the INSERT query.
pub async fn append_event(
    db: &DbStore,
    event_type: &str,
    _source: &str,
    project: Option<String>,
    payload: Option<&str>,
) -> Result<String> {
    let now = crate::db::now_rfc3339();
    let mut response = db
        .exec_params(
            "INSERT INTO events (project, kind, payload, created_at) \
             VALUES ($project, $kind, $payload, $now) \
             RETURNING id",
            libsql::params![
                project,
                event_type.to_owned(),
                payload.map(ToOwned::to_owned),
                now
            ],
        )
        .await?;
    match response.next().await? {
        // `events.id` is `INTEGER PRIMARY KEY AUTOINCREMENT`. Reading it as
        // `String` hits libsql's `unreachable!("invalid value type")` and takes
        // the whole process down with a panic — `kavach db event` crashed on
        // EVERY payload, including `{}`. Read the declared type, then render.
        Some(r) => Ok(r.get::<i64>(0)?.to_string()),
        None => Err(crate::error::Error::RecordNotFound("event create".into())),
    }
}
