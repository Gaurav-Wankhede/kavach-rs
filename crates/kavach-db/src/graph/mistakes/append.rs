// split: keyed-idempotent event upsert (mistake+loophole), not a request handler
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(SurrealValue)]
struct IdRow {
    id: RecordId,
}

/// Derives the stable, deterministic event name from its semantic identity.
/// Same shape as `cluster::derive_pattern_name` — first 8 hex chars of blake3.
fn event_key(prefix: &str, parts: &[&str], session_id: &str, turn: i32) -> String {
    let joined = parts.join("|");
    let hash = blake3::hash(format!("{joined}|{session_id}|{turn}").as_bytes());
    let hex = hash.to_hex();
    let short: String = hex.chars().take(8).collect();
    format!("{prefix}.{short}")
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
    turn: i32,
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
    let name = event_key(
        "mev",
        &[gate, correct_action, banned_sample],
        session_id,
        turn,
    );
    create_event(db, "mistake_event", &name, props).await
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
    turn: i32,
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
    let name = event_key("lev", &[dimension, site], session_id, turn);
    create_event(db, "loophole_event", &name, props).await
}

/// Shared idempotent event upsert keyed by `name`, so a re-file converges.
async fn create_event(
    db: &Surreal<Db>,
    entity_type: &str,
    name: &str,
    props: serde_json::Value,
) -> Result<RecordId> {
    let q = "UPSERT entity SET \
             entity_type = $etype, \
             name = $name, \
             properties = $props, \
             created_at = time::now() \
             WHERE entity_type = $etype AND name = $name \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("etype", entity_type.to_owned()))
        .bind(("name", name.to_owned()))
        .bind(("props", props))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("{entity_type} create empty")))
}

#[cfg(test)]
#[path = "append_test.rs"]
mod append_test;
