// Append-only mistake_event creation. No read-modify-write. No prose-hash key.
// Each event is unique; aggregation is via inbound instance_of edges on the
// anti_pattern centroid (Bug 1 + Bug 3 dissolved).
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
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
    embedding: Vec<f32>,
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
    });
    let q = "CREATE entity SET \
             entity_type = 'mistake_event', \
             name = rand::ulid(), \
             properties = $props, \
             embedding = $emb, \
             created_at = time::now() \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("props", props))
        .bind(("emb", embedding))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound("mistake_event create empty".into()))
}
