// Append-only mistake_event creation. No read-modify-write. No prose-hash key.
// Each event is unique; aggregation is via inbound instance_of edges on the
// anti_pattern centroid (Bug 1 + Bug 3 dissolved).
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(SurrealValue)]
struct IdRow {
    id: RecordId,
}

/// Creates an append-only mistake event record in the database.
///
/// # Errors
///
/// Returns an error if `gate` is empty, the query fails, or the database returns no record.
pub async fn append_mistake_event(
    db: &Surreal<Db>,
    gate: &str,
    correct_action: &str,
    banned_sample: &str,
    session_id: &str,
    project_slug: Option<&str>,
) -> Result<RecordId> {
    if gate.is_empty() {
        return Err(Error::Migration(
            "mistake_event: gate cannot be empty".into(),
        ));
    }
    let props = serde_json::json!({
        "gate": gate,
        "correct_action": correct_action,
        "banned_sample": banned_sample,
        "session_id": session_id,
        "project_slug": project_slug,
        "family": super::pattern::FAMILY_MISTAKE,
    });
    create_event(db, "mistake_event", props).await
}

/// Append-only loophole event — the umbrella's loophole half.
///
/// Same `entity` tier as a mistake_event, tagged `family='loophole'`, so one
/// ledger / recall path serves both. `dimension` is the agnostic lens
/// (injection/xss/memory-safety/…); `site` is the `file:line — hint` the lens scan
/// flagged. SOURCE: decision.loophole-mistake-umbrella.
///
/// # Errors
/// Returns an error if `dimension` is empty, the query fails, or no record returns.
pub async fn append_loophole_event(
    db: &Surreal<Db>,
    dimension: &str,
    site: &str,
    session_id: &str,
    project_slug: Option<&str>,
) -> Result<RecordId> {
    if dimension.is_empty() {
        return Err(Error::Migration(
            "loophole_event: dimension cannot be empty".into(),
        ));
    }
    let props = serde_json::json!({
        "gate": dimension,
        "site": site,
        "session_id": session_id,
        "project_slug": project_slug,
        "family": super::pattern::FAMILY_LOOPHOLE,
    });
    create_event(db, "loophole_event", props).await
}

/// Shared append-only event create over the `entity` table. Both families flow
/// through here so the row shape (ulid name, props, created_at) stays identical.
async fn create_event(
    db: &Surreal<Db>,
    entity_type: &str,
    props: serde_json::Value,
) -> Result<RecordId> {
    let q = "CREATE entity SET \
             entity_type = $etype, \
             name = rand::ulid(), \
             properties = $props, \
             created_at = time::now() \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("etype", entity_type.to_owned()))
        .bind(("props", props))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("{entity_type} create empty")))
}
