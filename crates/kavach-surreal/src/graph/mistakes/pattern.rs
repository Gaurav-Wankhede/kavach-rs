// Anti-pattern upsert — L0.5 nodes that aggregate mistake_events by gate+
// correct_action centroid. Stored on entity table with entity_type='anti_pattern'.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

/// Upsert an anti-pattern entity with a centroid embedding.
///
/// # Errors
/// Returns `Error::Migration` if the name is empty, or `Error::RecordNotFound` if the upsert query returns no rows.
pub async fn upsert_anti_pattern(
    db: &Surreal<Db>,
    name: &str,
    gate: &str,
    correct_action: &str,
    centroid_embedding: &[f32],
) -> Result<RecordId> {
    #[derive(SurrealValue)]
    struct IdRow {
        id: RecordId,
    }

    if name.is_empty() {
        return Err(Error::Migration("anti_pattern name cannot be empty".into()));
    }
    let props = serde_json::json!({
        "gate": gate,
        "correct_action": correct_action,
    });
    let q = "UPSERT entity \
             SET entity_type = 'anti_pattern', name = $name, properties = $props, \
                 embedding = $emb, updated_at = time::now() \
             WHERE entity_type = 'anti_pattern' AND name = $name \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("name", name.to_owned()))
        .bind(("props", props))
        .bind(("emb", centroid_embedding.to_vec()))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("anti_pattern upsert empty: {name}")))
}
