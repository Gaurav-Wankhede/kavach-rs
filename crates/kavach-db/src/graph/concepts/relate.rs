// Ontology edges between L0 concepts. Edge name validated against
// ALLOWED_ONTOLOGY_RELS, then bound as a parameter (never interpolated).
use crate::error::{Error, Result};
use crate::graph::dynamic::{ALLOWED_ONTOLOGY_RELS, is_bridge_rel};
use crate::db::DbStore;

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
/// Both endpoints are upserted into `entities` (idempotent), then the typed
/// edge is written into `entity_edges` — one transaction so a partial write
/// can never leave a dangling edge.
///
/// # Errors
/// Propagates `Error::Migration` if the edge is not an ontology edge or if concept names are empty.
pub async fn relate_concepts(
    db: &DbStore,
    from_name: &str,
    edge: &str,
    to_name: &str,
) -> Result<()> {
    validate_ontology(edge)?;
    if from_name.is_empty() || to_name.is_empty() {
        return Err(Error::Migration("concept names cannot be empty".into()));
    }
    let src_id = format!("concept:{from_name}");
    let tgt_id = format!("concept:{to_name}");
    let txn = db.conn.transaction().await?;
    // `execute` (not `query`) so a failed INSERT surfaces immediately instead
    // of being deferred into unconsumed Rows (§no_silent_failure).
    txn.execute(
        "INSERT INTO entities (id, entity_type, name, properties) \
         VALUES (?1, 'concept', ?2, NULL) \
         ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        libsql::params![src_id.clone(), from_name.to_owned()],
    )
    .await?;
    txn.execute(
        "INSERT INTO entities (id, entity_type, name, properties) \
         VALUES (?1, 'concept', ?2, NULL) \
         ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        libsql::params![tgt_id.clone(), to_name.to_owned()],
    )
    .await?;
    txn.execute(
        "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
         VALUES (?1, ?2, ?3, 1.0, ?4) \
         ON CONFLICT(from_id, rel_type, to_id) DO UPDATE SET weight = 1.0",
        libsql::params![src_id, edge.to_owned(), tgt_id, crate::db::now_rfc3339()],
    )
    .await?;
    txn.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "relate_test.rs"]
mod relate_test;
