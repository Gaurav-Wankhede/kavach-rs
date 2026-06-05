// Compile-time dispatch over (table, edge) pairs — 20 const queries.
// Same pattern as `fix-Bash-sql-guard-format-on-validated-arrow`.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::RecordId;

macro_rules! q_bridge {
    ($table:literal, $edge:literal) => {
        concat!(
            "BEGIN TRANSACTION; LET $src = (SELECT id FROM ",
            $table,
            " WHERE entry_key = $src_key LIMIT 1)[0].id; ",
            "LET $tgt = (UPSERT entity SET entity_type = 'concept', ",
            "name = $concept_name, updated_at = time::now() ",
            "WHERE entity_type = 'concept' AND name = $concept_name RETURN id)[0].id; ",
            "LET $rel = (RELATE $src->",
            $edge,
            "->$tgt SET weight = 1.0 RETURN id)[0].id; ",
            "COMMIT TRANSACTION; RETURN $rel;"
        )
    };
}

fn pick_query(table: &str, edge: &str) -> Option<&'static str> {
    match (table, edge) {
        ("roadmap", "implements") => Some(q_bridge!("roadmap", "implements")),
        ("roadmap", "discusses") => Some(q_bridge!("roadmap", "discusses")),
        ("roadmap", "references_concept") => Some(q_bridge!("roadmap", "references_concept")),
        ("roadmap", "violates") => Some(q_bridge!("roadmap", "violates")),
        ("decision", "implements") => Some(q_bridge!("decision", "implements")),
        ("decision", "discusses") => Some(q_bridge!("decision", "discusses")),
        ("decision", "references_concept") => Some(q_bridge!("decision", "references_concept")),
        ("decision", "violates") => Some(q_bridge!("decision", "violates")),
        ("research", "implements") => Some(q_bridge!("research", "implements")),
        ("research", "discusses") => Some(q_bridge!("research", "discusses")),
        ("research", "references_concept") => Some(q_bridge!("research", "references_concept")),
        ("research", "violates") => Some(q_bridge!("research", "violates")),
        ("pattern", "implements") => Some(q_bridge!("pattern", "implements")),
        ("pattern", "discusses") => Some(q_bridge!("pattern", "discusses")),
        ("pattern", "references_concept") => Some(q_bridge!("pattern", "references_concept")),
        ("pattern", "violates") => Some(q_bridge!("pattern", "violates")),
        ("app_spec", "implements") => Some(q_bridge!("app_spec", "implements")),
        ("app_spec", "discusses") => Some(q_bridge!("app_spec", "discusses")),
        ("app_spec", "references_concept") => Some(q_bridge!("app_spec", "references_concept")),
        ("app_spec", "violates") => Some(q_bridge!("app_spec", "violates")),
        (_, _) => None,
    }
}

/// Create a bridge linking an entity to a concept via an edge.
///
/// # Errors
/// Propagates `Error::Migration` when inputs are empty or the (table, edge) pair is invalid.
/// Propagates `Error::RecordNotFound` when the bridge record cannot be retrieved.
pub async fn bridge_to_concept(
    db: &Surreal<Db>,
    src_table: &str,
    src_key: &str,
    edge: &str,
    concept_name: &str,
) -> Result<RecordId> {
    if src_key.is_empty() || concept_name.is_empty() {
        return Err(Error::Migration("bridge endpoints cannot be empty".into()));
    }
    let Some(q) = pick_query(src_table, edge) else {
        return Err(Error::Migration(format!(
            "invalid (table, edge): ({src_table}, {edge})"
        )));
    };
    let mut resp = db
        .query(q)
        .bind(("src_key", src_key.to_owned()))
        .bind(("concept_name", concept_name.to_owned()))
        .await?;
    let last = resp.num_statements().saturating_sub(1);
    let raw: Option<RecordId> = resp.take(last)?;
    raw.map_or_else(
        || {
            Err(Error::RecordNotFound(format!(
                "bridge create: {src_table}/{src_key}"
            )))
        },
        Ok,
    )
}
