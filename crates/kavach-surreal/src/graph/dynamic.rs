// split: intentional - dynamic graph ops (string rel_type, upsert-by-name) for engine gates
// SurrealDB-backed dynamic graph helpers — mirror kavach-db graph_entity + graph
// add_relationship/get_related shapes. The fixed RelationType enum in
// graph::types::RelationType is preferred for new code; this module exists to
// migrate engine gates that use dynamic rel_types like "uses_skill",
// "cross_invoke", "INVOKE" without expanding the typed enum.
// sql-safe: bound params; rel_type validated against an allowlist before use.
use crate::error::{Error, Result};
use crate::graph::types::Entity;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

/// Allowlist of relation names accepted by dynamic helpers.
/// Adding a new `rel_type` here is the only path for engine gates to introduce one.
/// Underscored names match the `SurrealDB` graph edge table identifiers.
const ALLOWED_RELS: &[&str] = &[
    // Workflow / project-DAG edges (original set)
    "contains",
    "depends_on",
    "modifies",
    "references",
    "mentions",
    "works_on",
    "owns",
    "uses_skill",
    "cross_invoke",
    "invoke",
    "uses_pattern",
    "in_scope",
    "uses_algorithm",
    "session_uses_skill",
    "solves",
    // Ontology edges (L0 concept <-> L0 concept) — knowledge-graph structure.
    "is_a",
    "part_of",
    "prerequisite_of",
    "alternative_to",
    "composes",
    "mitigates",
    "instance_of",
    "subsumes",
    // Bridge edges (L1 project_entity -> L0 concept) — cross-project glue.
    "implements",
    "discusses",
    "references_concept",
    "violates",
    // Mistake-tier edges (L3 mistake_event -> {anti_pattern, gate, session, concept})
    "fired_gate",
    "triggered_in_session",
    "correct_action_ref",
    // Citation-tier edges (unified docs-awareness DAG):
    //   {decision,research,pattern,mistake,roadmap} -cite-> citation
    //   citation -parent-> citation, citation -depends_on-> citation
    "cite",
    "parent",
];

/// Subset of `ALLOWED_RELS` valid for citation-DAG relations. `cite` anchors a
/// knowledge node to its official-docs citation; `parent`/`depends_on` link
/// citations into the documentation hierarchy DAG.
pub(crate) const ALLOWED_CITATION_RELS: &[&str] = &["cite", "parent", "depends_on"];

/// Returns true if `rel` is a citation-DAG edge.
#[must_use]
pub fn is_citation_rel(rel: &str) -> bool {
    ALLOWED_CITATION_RELS.contains(&rel)
}

/// Subset of `ALLOWED_RELS` valid for ontology relations (concept <-> concept).
/// Used by `graph::concepts` to reject workflow edges on concept-only relate calls.
pub(crate) const ALLOWED_ONTOLOGY_RELS: &[&str] = &[
    "is_a",
    "part_of",
    "prerequisite_of",
    "alternative_to",
    "composes",
    "mitigates",
    "instance_of",
    "subsumes",
];

/// Subset of `ALLOWED_RELS` valid for L1->L0 bridges (`project_entity` -> concept).
/// Consumed by `graph::concepts::relate` to reject bridge edges on ontology relate.
pub(crate) const ALLOWED_BRIDGE_RELS: &[&str] =
    &["implements", "discusses", "references_concept", "violates"];

/// Returns true if `rel` is a bridge edge (L1->L0). Used by concept relate
/// helpers to produce precise error messages when the caller passes a bridge
/// edge to an ontology-only function.
pub(crate) fn is_bridge_rel(rel: &str) -> bool {
    ALLOWED_BRIDGE_RELS.contains(&rel)
}

fn validate_rel(rel: &str) -> Result<()> {
    if ALLOWED_RELS.contains(&rel) {
        Ok(())
    } else {
        Err(Error::Migration(format!(
            "rel_type '{rel}' not in allowlist; add to graph::dynamic::ALLOWED_RELS"
        )))
    }
}

#[derive(surrealdb_types::SurrealValue)]
struct EntityIdRow {
    id: RecordId,
}

/// Find an entity by (`entity_type`, name); insert it if absent. Returns its id.
///
/// # Errors
/// `Error::Surreal` on SELECT or CREATE failure; `Error::RecordNotFound`
/// when the CREATE returns no id row.
pub async fn upsert_entity(db: &Surreal<Db>, entity_type: &str, name: &str) -> Result<RecordId> {
    let find_q = "SELECT id FROM entity \
                  WHERE entity_type = $type AND name = $name LIMIT 1";
    let mut response = db
        .query(find_q)
        .bind(("type", entity_type.to_owned()))
        .bind(("name", name.to_owned()))
        .await?;
    let existing: Option<EntityIdRow> = response.take(0)?;
    if let Some(row) = existing {
        return Ok(row.id);
    }
    let create_q = "CREATE entity SET entity_type = $type, name = $name RETURN id";
    let mut resp = db
        .query(create_q)
        .bind(("type", entity_type.to_owned()))
        .bind(("name", name.to_owned()))
        .await?;
    let row: Option<EntityIdRow> = resp.take(0)?;
    row.map(|ir| ir.id)
        .ok_or_else(|| Error::RecordNotFound("entity create returned no id".into()))
}

/// List entities, optionally filtered by `entity_type`. Caps at `LIST_ENTITIES_MAX`.
const LIST_ENTITIES_MAX: i64 = 5_000;

/// List entities, optionally filtered by `entity_type`. Caps at `LIST_ENTITIES_MAX`.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_entities(db: &Surreal<Db>, entity_type: Option<&str>) -> Result<Vec<Entity>> {
    let q = match entity_type {
        Some(_) => {
            "SELECT id, entity_type, name, properties, content_hash, project FROM entity \
                    WHERE entity_type = $type LIMIT $limit"
        }
        None => {
            "SELECT id, entity_type, name, properties, content_hash, project FROM entity \
                 LIMIT $limit"
        }
    };
    let mut response = match entity_type {
        Some(et) => {
            db.query(q)
                .bind(("type", et.to_owned()))
                .bind(("limit", LIST_ENTITIES_MAX))
                .await?
        }
        None => db.query(q).bind(("limit", LIST_ENTITIES_MAX)).await?,
    };
    let entities: Vec<Entity> = response.take(0)?;
    Ok(entities)
}

/// Find an entity by (`entity_type`, name). Returns None if absent.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn find_entity(
    db: &Surreal<Db>,
    entity_type: &str,
    name: &str,
) -> Result<Option<Entity>> {
    let q = "SELECT id, entity_type, name, properties, content_hash, project FROM entity \
             WHERE entity_type = $type AND name = $name LIMIT 1";
    let mut response = db
        .query(q)
        .bind(("type", entity_type.to_owned()))
        .bind(("name", name.to_owned()))
        .await?;
    let row: Option<Entity> = response.take(0)?;
    Ok(row)
}

/// Add a relationship between two entities by string `rel_type`.
/// `rel_type` must be in `ALLOWED_RELS` — unknown names error to prevent typos.
///
/// # Errors
/// `Error::RecordNotFound` when `rel_type` is not in `ALLOWED_RELS`;
/// `Error::Surreal` from the RELATE.
pub async fn relate_dynamic(
    db: &Surreal<Db>,
    from: &RecordId,
    to: &RecordId,
    rel_type: &str,
    weight: f64,
) -> Result<()> {
    validate_rel(rel_type)?;
    // SurrealDB RELATE statement requires the rel name as a bare identifier;
    // we cannot bind it as a parameter. The allowlist check above guarantees
    // rel_type is one of a fixed set of literals, so format!() is sql-safe.
    let q = format!("RELATE $from->{rel_type}->$to SET weight = $weight");
    db.query(q)
        .bind(("from", from.clone()))
        .bind(("to", to.clone()))
        .bind(("weight", weight))
        .await?;
    Ok(())
}

/// Relate a knowledge node to a citation (or a citation to a parent/dependency).
/// Rejects any edge not in `ALLOWED_CITATION_RELS` so a workflow/ontology edge
/// cannot leak into the citation DAG.
///
/// # Errors
/// `Error::Migration` when `rel_type` is not a citation edge; `Error::Surreal`
/// from the RELATE.
pub async fn relate_citation(
    db: &Surreal<Db>,
    from: &RecordId,
    to: &RecordId,
    rel_type: &str,
    weight: f64,
) -> Result<()> {
    if !is_citation_rel(rel_type) {
        return Err(Error::Migration(format!(
            "rel_type '{rel_type}' is not a citation edge (cite|parent|depends_on)"
        )));
    }
    relate_dynamic(db, from, to, rel_type, weight).await
}

/// Single-query traversal: in ONE `SurrealDB` round-trip via the `<-cite` graph
/// arrow, return the record ids of every node that cites `citation` (no N+1).
///
/// # Errors
/// Propagates `Error::Surreal` from the query.
pub async fn traverse_with_citations(
    db: &Surreal<Db>,
    citation: &RecordId,
) -> Result<Vec<RecordId>> {
    let ids: Vec<RecordId> = db
        .query("SELECT VALUE in FROM $cit<-cite")
        .bind(("cit", citation.clone()))
        .await?
        .take(0)?;
    Ok(ids)
}

/// Forward complement of `traverse_with_citations`: in ONE `SurrealDB` round-trip
/// via the `->cite` arrow, return the citation record ids that `node` cites.
///
/// Lets a mistake/decision/roadmap node reach its merged citations without a
/// second query — the read half of the non-destructive C6 merge.
///
/// # Errors
/// Propagates `Error::Surreal` from the query.
pub async fn citations_cited_by(db: &Surreal<Db>, node: &RecordId) -> Result<Vec<RecordId>> {
    let ids: Vec<RecordId> = db
        .query("SELECT VALUE out FROM $node->cite")
        .bind(("node", node.clone()))
        .await?
        .take(0)?;
    Ok(ids)
}

/// One row of the related-entity result set: outgoing edge → target entity.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct RelatedRow {
    pub rel_type: String,
    pub weight: f64,
    pub target: Entity,
}

/// One edge whose both endpoints fall inside a given node set. Endpoint ids are
/// stringified record ids matching the form the KG renderer keys nodes by.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct EdgeRow {
    pub from: String,
    pub to: String,
    pub rel_type: String,
}

/// Return every allowed edge whose endpoints are both in `node_ids`.
///
/// These are the edges internal to the visible node set, so the graph view never
/// draws a line to an off-canvas node. Each source node is traversed per relation
/// (the only way to recover the relation name), then targets are filtered to the
/// set — keying endpoints exactly as the KG renderer does so they resolve.
///
/// # Errors
/// Propagates `Error::Surreal` from any per-relation SELECT.
pub async fn list_edges_among(db: &Surreal<Db>, node_ids: &[RecordId]) -> Result<Vec<EdgeRow>> {
    if node_ids.len() < 2 {
        return Ok(Vec::new());
    }
    // Membership set keyed the same way endpoints are stringified, so the
    // post-filter and the renderer agree on node identity.
    let allowed: std::collections::HashSet<String> =
        node_ids.iter().map(|id| format!("{id:?}")).collect();

    let mut out: Vec<EdgeRow> = Vec::new();
    for node in node_ids {
        for &rel in ALLOWED_RELS {
            // sql-safe: rel is a compile-time const from ALLOWED_RELS; the source
            // node is bound. Traverse outgoing edges via the graph operator (the
            // codebase-proven form — `get_related` uses the same `->rel->`), then
            // keep only edges whose target is also in the visible set.
            let q = format!("SELECT out AS target FROM $node->{rel}");
            let mut response = db.query(q).bind(("node", node.clone())).await?;
            let targets: Vec<TargetId> = match response.take(0) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let from = format!("{node:?}");
            for t in targets {
                let to = format!("{:?}", t.target);
                if allowed.contains(&to) {
                    out.push(EdgeRow {
                        from: from.clone(),
                        to,
                        rel_type: rel.to_owned(),
                    });
                }
            }
        }
    }
    Ok(out)
}

/// A single outgoing-edge target id pulled from a `->rel->` traversal.
#[derive(Debug, Clone, Deserialize, SurrealValue)]
struct TargetId {
    target: RecordId,
}

/// Return outgoing edges from `from` across all `ALLOWED_RELS`, paired with
/// the target entity. Limit caps the per-rel-type fan-out.
///
/// # Errors
/// Propagates `Error::Surreal` from any per-relation SELECT.
pub async fn get_related(
    db: &Surreal<Db>,
    from: &RecordId,
    limit: usize,
) -> Result<Vec<RelatedRow>> {
    let mut out: Vec<RelatedRow> = Vec::new();
    for &rel in ALLOWED_RELS {
        // sql-safe: rel comes from compile-time const ALLOWED_RELS only.
        let q = format!(
            "SELECT '{rel}' AS rel_type, weight, out.* AS target \
             FROM type::record($tb, $id)->{rel} LIMIT $limit"
        );
        let mut response = db
            .query(q)
            .bind(("tb", format!("{:?}", &from.table)))
            .bind(("id", format!("{:?}", &from.key)))
            .bind(("limit", i64::try_from(limit).unwrap_or(i64::MAX)))
            .await?;
        let rows: Vec<RelatedRow> = match response.take(0) {
            Ok(r) => r,
            Err(_) => continue,
        };
        out.extend(rows);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "dynamic_citation_test.rs"]
mod citation_edge_tests;
