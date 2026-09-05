use crate::error::{Error, Result};
use crate::turso::TursoStore;

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
    db: &TursoStore,
    name: &str,
    gate: &str,
    correct_action: &str,
) -> Result<String> {
    upsert_anti_pattern_with_family(db, name, gate, correct_action, FAMILY_MISTAKE).await
}

/// As [`upsert_anti_pattern`] but tags `properties.family` ({`mistake`|`loophole`})
/// so mistakes and loopholes share one graph tier under the umbrella.
///
/// # Errors
/// Returns `Error::Migration` if the name is empty, or `Error::RecordNotFound` if the upsert query returns no rows.
pub async fn upsert_anti_pattern_with_family(
    db: &TursoStore,
    name: &str,
    gate: &str,
    correct_action: &str,
    family: &str,
) -> Result<String> {
    if name.is_empty() {
        return Err(Error::Migration("anti_pattern name cannot be empty".into()));
    }
    let props = serde_json::json!({
        "gate": gate,
        "correct_action": correct_action,
        "family": family,
    });
    let props_json = serde_json::to_string(&props)?;
    let id = format!("anti_pattern:{name}");
    let mut resp = db
        .exec_params(
            "INSERT INTO entities (id, entity_type, name, properties) \
             VALUES ($id, 'anti_pattern', $name, $props) \
             ON CONFLICT(id) DO UPDATE SET \
                 name = excluded.name, properties = excluded.properties \
             RETURNING id",
            libsql::params![id.clone(), name.to_owned(), props_json],
        )
        .await?;
    match resp.next().await? {
        Some(_) => Ok(id),
        None => Err(Error::RecordNotFound(format!(
            "anti_pattern upsert empty: {name}"
        ))),
    }
}
