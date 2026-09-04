// Anti-pattern upsert — L0.5 nodes that aggregate mistake_events by gate+
// correct_action centroid. Stored on entity table with entity_type='anti_pattern'.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

/// The two anti-pattern families that share this graph tier (the umbrella):
/// a committed error (`mistake`) and a predicted risk surface (`loophole`).
/// SOURCE: decision.loophole-mistake-umbrella.
pub const FAMILY_MISTAKE: &str = "mistake";
/// The loophole family — a predicted risk-class, same graph tier as a mistake.
pub const FAMILY_LOOPHOLE: &str = "loophole";

/// Upsert an anti-pattern entity keyed by its content-derived `name`.
///
/// Back-compat shim: defaults `family` to `mistake`. New callers that need the
/// umbrella tag use [`upsert_anti_pattern_with_family`].
///
/// # Errors
/// Returns `Error::Migration` if the name is empty, or `Error::RecordNotFound` if the upsert query returns no rows.
pub async fn upsert_anti_pattern(
    db: &Surreal<Db>,
    name: &str,
    gate: &str,
    correct_action: &str,
) -> Result<RecordId> {
    upsert_anti_pattern_with_family(db, name, gate, correct_action, FAMILY_MISTAKE).await
}

/// As [`upsert_anti_pattern`] but tags `properties.family` ({`mistake`|`loophole`})
/// so mistakes and loopholes share one graph tier under the umbrella.
///
/// # Errors
/// Returns `Error::Migration` if the name is empty, or `Error::RecordNotFound` if the upsert query returns no rows.
pub async fn upsert_anti_pattern_with_family(
    db: &Surreal<Db>,
    name: &str,
    gate: &str,
    correct_action: &str,
    family: &str,
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
        "family": family,
    });
    let q = "UPSERT entity \
             SET entity_type = 'anti_pattern', name = $name, properties = $props, \
                 updated_at = time::now() \
             WHERE entity_type = 'anti_pattern' AND name = $name \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("name", name.to_owned()))
        .bind(("props", props))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("anti_pattern upsert empty: {name}")))
}
