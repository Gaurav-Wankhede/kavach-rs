use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(SurrealValue)]
pub(super) struct EventRow {
    pub id: RecordId,
}

/// Delete events older than `days` days. Returns count deleted.
///
/// # Errors
/// Propagates `Error::Surreal` from the DELETE query.
pub async fn rotate_events(db: &Surreal<Db>, days: i64) -> Result<usize> {
    let query =
        "DELETE event WHERE created_at < time::now() - duration::from::days($days) RETURN BEFORE"; // doctor:ok
    let mut response = db.query(query).bind(("days", days)).await?;
    let deleted: Vec<serde_json::Value> = response.take(0)?;
    Ok(deleted.len())
}

/// Append an event row.
///
/// # Errors
/// Propagates `Error::Surreal` from the CREATE query.
pub async fn append_event(
    db: &Surreal<Db>,
    event_type: &str,
    source: &str,
    project: Option<RecordId>,
    payload: Option<&str>,
) -> Result<RecordId> {
    let payload_value: Option<serde_json::Value> = payload.map(|p| {
        serde_json::from_str(p).unwrap_or_else(|_| serde_json::Value::String(p.to_owned()))
    });
    let query = "CREATE event SET event_type = $event_type, source = $source, \
                 project = $project, payload = $payload, created_at = time::now() RETURN AFTER";
    let mut response = db
        .query(query)
        .bind(("event_type", event_type.to_owned()))
        .bind(("source", source.to_owned()))
        .bind(("project", project))
        .bind(("payload", payload_value))
        .await?;
    let result: Option<EventRow> = response.take(0)?;
    match result {
        Some(e) => Ok(e.id),
        None => Err(crate::error::Error::RecordNotFound("event create".into())),
    }
}
