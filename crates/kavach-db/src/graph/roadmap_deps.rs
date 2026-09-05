//! Kanban dependency edges as FIRST-CLASS graph edges.
//!
//! Historically the kanban DAG lived only as `DEPENDS_ON:` TEXT
//! lines in a card's content, parsed in Rust and walked by a hand-rolled DFS
//! (`kavach-rpc roadmap::readiness::cycle`). This module mirrors those declared
//! deps into real `roadmap_card --depends_on--> roadmap_card` edges so the
//! dependency DAG is queryable with SQL — unifying the kanban DAG with the
//! concept/flow knowledge graph.
//!
//! The card-content `DEPENDS_ON:` line stays the human-authored SOURCE; the
//! edge is its queryable PROJECTION. `mirror_card_deps` is idempotent: it
//! deletes the card's existing out-edges then re-creates them from the current
//! dep list, so re-running on an edited card converges (last-writer wins)
//! without duplicate edges.
use crate::error::{Error, Result};
use crate::turso::TursoStore;

/// `entity_type` tag for a kanban card node in the shared `entities` table.
/// Distinct from `concept`/`flow_step` so a card's edges never collide with the
/// knowledge graph.
const CARD_KIND: &str = "roadmap_card";

/// Validate a card key before it is used as a bound param value. Keys are
/// authored as kanban slugs (alphanumeric + `_`/`-`/`.`); rejecting anything
/// else keeps a hostile/malformed key out of the graph at the edge. Empty is
/// rejected too.
fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(Error::Migration("roadmap card key cannot be empty".into()));
    }
    if key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Ok(());
    }
    Err(Error::Migration(format!(
        "roadmap card key '{key}' has illegal chars (allowed: alphanumeric _ - .)"
    )))
}

/// Mirror one card's declared dependency keys into `depends_on` edges.
///
/// Idempotent: clears `card`'s existing out-edges, then inserts `card -> dep`
/// for each `dep`. The edge direction is prerequisite-facing the SAME way the
/// scheduler reads it — `card` depends on each `dep`, so `dep` must finish
/// first. Dep keys are NOT required to exist as cards yet; the anchor is
/// upserted so the edge is never lost, matching the fail-safe "unknown dep
/// blocks" tolerance of the text path.
///
/// # Errors
/// Propagates `Error::Migration` on an invalid key and `Error::Turso` on any
/// query failure. Runs in one transaction so a mid-op crash leaves no partial
/// edge set.
pub async fn mirror_card_deps(db: &TursoStore, card_key: &str, deps: &[String]) -> Result<()> {
    validate_key(card_key)?;
    for d in deps {
        validate_key(d)?;
    }
    let now = crate::turso::now_rfc3339();
    let txn = db.conn.transaction().await?;
    // `execute` (not `query`) so a failed INSERT/DELETE surfaces immediately
    // instead of being deferred into unconsumed Rows (§no_silent_failure).
    // Upsert the card node.
    txn.execute(
        "INSERT INTO entities (id, entity_type, name) \
         VALUES ('roadmap_card:' || ?1, ?2, ?1) \
         ON CONFLICT(id) DO NOTHING",
        libsql::params![card_key.to_owned(), CARD_KIND.to_owned()],
    )
    .await?;
    // Clear existing out-edges (idempotent convergence).
    txn.execute(
        "DELETE FROM entity_edges WHERE from_id = 'roadmap_card:' || ?1 AND rel_type = 'depends_on'",
        libsql::params![card_key.to_owned()],
    )
    .await?;
    for d in deps {
        txn.execute(
            "INSERT INTO entities (id, entity_type, name) \
             VALUES ('roadmap_card:' || ?1, ?2, ?1) \
             ON CONFLICT(id) DO NOTHING",
            libsql::params![d.to_owned(), CARD_KIND.to_owned()],
        )
        .await?;
        txn.execute(
            "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
             VALUES ('roadmap_card:' || ?1, 'depends_on', 'roadmap_card:' || ?2, 1.0, ?3) \
             ON CONFLICT(from_id, rel_type, to_id) DO NOTHING",
            libsql::params![card_key.to_owned(), d.to_owned(), now.clone()],
        )
        .await?;
    }
    txn.commit().await?;
    Ok(())
}

/// SQL-native cycle check: does `card_key` participate in a `depends_on` cycle?
///
/// Walks the transitive prerequisite closure with a recursive CTE, then asks
/// whether the start node is reachable from itself. This is the SQL equivalent
/// of the Rust three-color DFS in `kavach-rpc roadmap::readiness::cycle::is_in_cycle`
/// — including the self-dependency boundary case (`A` depends on `A`).
///
/// # Errors
/// Propagates `Error::Migration` on an invalid key, `Error::Turso` on query
/// failure.
pub async fn is_in_cycle_sql(db: &TursoStore, card_key: &str) -> Result<bool> {
    validate_key(card_key)?;
    let start = format!("roadmap_card:{card_key}");
    let mut res = db
        .exec_params(
            "WITH RECURSIVE closure(node) AS ( \
               SELECT to_id FROM entity_edges \
               WHERE from_id = ?1 AND rel_type = 'depends_on' \
               UNION \
               SELECT e.to_id FROM entity_edges e \
               JOIN closure c ON e.from_id = c.node \
               WHERE e.rel_type = 'depends_on' \
             ) \
             SELECT 1 FROM closure WHERE node = ?1 LIMIT 1",
            libsql::params![start],
        )
        .await?;
    Ok(res.next().await?.is_some())
}

#[cfg(test)]
#[path = "roadmap_deps_test.rs"]
mod tests;
