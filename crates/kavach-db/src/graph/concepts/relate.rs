// Ontology edges between L0 concepts. Edge name validated against
// ALLOWED_ONTOLOGY_RELS, then interpolated as a bare identifier.
// sql-safe: edge string comes from compile-time const ALLOWED_ONTOLOGY_RELS
// after validate_ontology() rejects anything outside the allow-list. Same
// escape-hatch pattern used by graph/dynamic.rs::relate_dynamic — installed
// SurrealDB parser does NOT accept type::table($edge) in RELATE edge slot
// despite docs example (see decision:concept-relate-runtime-fix).
use crate::error::{Error, Result};
use crate::graph::dynamic::{ALLOWED_ONTOLOGY_RELS, is_bridge_rel};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

fn validate_ontology(rel: &str) -> Result<()> {
    if ALLOWED_ONTOLOGY_RELS.contains(&rel) {
        return Ok(());
    }
    if is_bridge_rel(rel) {
        return Err(Error::Migration(format!(
            "rel '{rel}' is a bridge edge (L1->L0); use graph::bridges (Iter 2), not relate_concepts"
        )));
    }
    Err(Error::Migration(format!(
        "rel '{rel}' is not an ontology edge; valid: {ALLOWED_ONTOLOGY_RELS:?}"
    )))
}

/// Relates two L0 concepts via a validated ontology edge.
///
/// # Errors
/// Propagates `Error::Migration` if the edge is not an ontology edge or if concept names are empty.
pub async fn relate_concepts(
    db: &Surreal<Db>,
    from_name: &str,
    edge: &str,
    to_name: &str,
) -> Result<()> {
    validate_ontology(edge)?;
    if from_name.is_empty() || to_name.is_empty() {
        return Err(Error::Migration("concept names cannot be empty".into()));
    }
    let src_q = "UPSERT entity SET entity_type = 'concept', name = $from_name, \
                 updated_at = time::now() WHERE entity_type = 'concept' AND name = $from_name \
                 RETURN id";
    let tgt_q = "UPSERT entity SET entity_type = 'concept', name = $to_name, \
                 updated_at = time::now() WHERE entity_type = 'concept' AND name = $to_name \
                 RETURN id";
    let src_row: Option<serde_json::Value> = db
        .query(src_q)
        .bind(("from_name", from_name.to_owned()))
        .await?
        .take(0)?;
    let src_id = src_row
        .and_then(|v| v.get("id").cloned())
        .ok_or_else(|| Error::Migration(format!("concept '{from_name}' not found after upsert")))?;
    let tgt_row: Option<serde_json::Value> = db
        .query(tgt_q)
        .bind(("to_name", to_name.to_owned()))
        .await?
        .take(0)?;
    let tgt_id = tgt_row
        .and_then(|v| v.get("id").cloned())
        .ok_or_else(|| Error::Migration(format!("concept '{to_name}' not found after upsert")))?;
    let relate_q = format!("RELATE $src->{edge}->$tgt SET weight = 1.0");
    db.query(relate_q)
        .bind(("src", src_id))
        .bind(("tgt", tgt_id))
        .await?;
    Ok(())
}
