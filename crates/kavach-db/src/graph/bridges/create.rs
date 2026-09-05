// Compile-time dispatch over (table, edge) pairs — 20 const queries.
// Same pattern as `fix-Bash-sql-guard-format-on-validated-arrow`.
use crate::error::{Error, Result};
use crate::turso::TursoStore;

/// Bridge edge names allowed between a memory row and a concept.
const ALLOWED_EDGES: &[&str] = &["implements", "discusses", "references_concept", "violates"];

/// Bridge source tables (memory categories) allowed.
const ALLOWED_TABLES: &[&str] = &["roadmap", "decision", "research", "pattern", "app_spec"];

/// Create a bridge linking an entity to a concept via an edge.
///
/// The source memory row must exist (`memories`); the concept is upserted into
/// `entities`; the typed edge is written into `entity_edges` with the canonical
/// `memory:<qname>` / `concept:<name>` ids. One transaction per bridge so a
/// partial write can never leave a dangling edge.
///
/// # Errors
/// Propagates `Error::Migration` when inputs are empty or the (table, edge) pair is invalid.
/// Propagates `Error::RecordNotFound` when the source memory row does not exist.
pub async fn bridge_to_concept(
    db: &TursoStore,
    src_table: &str,
    src_key: &str,
    edge: &str,
    concept_name: &str,
) -> Result<String> {
    if src_key.is_empty() || concept_name.is_empty() {
        return Err(Error::Migration("bridge endpoints cannot be empty".into()));
    }
    if !ALLOWED_TABLES.contains(&src_table) || !ALLOWED_EDGES.contains(&edge) {
        return Err(Error::Migration(format!(
            "invalid (table, edge): ({src_table}, {edge})"
        )));
    }
    let src_qname = format!("{src_table}:{src_key}");
    let concept_id = format!("concept:{concept_name}");
    let txn = db.conn.transaction().await?;
    // Source must exist as a memory row — fail closed rather than bridge to air.
    let mut src = txn
        .query(
            "SELECT 1 FROM memories WHERE project = ?1 AND category = ?2 AND entry_key = ?3 LIMIT 1",
            libsql::params![src_table.to_owned(), src_table.to_owned(), src_key.to_owned()],
        )
        .await?;
    if src.next().await?.is_none() {
        return Err(Error::RecordNotFound(format!(
            "bridge source missing: {src_table}/{src_key}"
        )));
    }
    // Concept upsert (idempotent). `execute` (not `query`) so a failed INSERT
    // surfaces immediately instead of being deferred into unconsumed Rows.
    txn.execute(
        "INSERT INTO entities (id, entity_type, name, properties) \
         VALUES (?1, 'concept', ?2, NULL) \
         ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        libsql::params![concept_id.clone(), concept_name.to_owned()],
    )
    .await?;
    // Typed edge.
    txn.execute(
        "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
         VALUES (?1, ?2, ?3, 1.0, ?4) \
         ON CONFLICT(from_id, rel_type, to_id) DO UPDATE SET weight = 1.0",
        libsql::params![
            src_qname.clone(),
            edge.to_owned(),
            concept_id.clone(),
            crate::turso::now_rfc3339()
        ],
    )
    .await?;
    txn.commit().await?;
    Ok(format!("{src_qname}->{edge}->{concept_id}"))
}
