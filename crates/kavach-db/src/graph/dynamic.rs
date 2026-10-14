use crate::db::DbStore;
// split: intentional - dynamic graph ops (string rel_type, upsert-by-name) for engine gates
// SQL-backed dynamic graph helpers — delegate to the `GraphStore` trait
// (database/graph.rs) which models entities + edges as plain tables. The fixed
// RelationType enum in graph::types::RelationType is preferred for new code;
// this module exists to migrate engine gates that use dynamic rel_types like
// "uses_skill", "cross_invoke", "INVOKE" without expanding the typed enum.
// sql-safe: bound params; rel_type validated against an allowlist before use.
use crate::error::{Error, Result};
use crate::graph::types::Entity;
use crate::db::graph::{GraphEntity, GraphStore};
use serde::{Deserialize, Serialize};

/// Allowlist of relation names accepted by dynamic helpers.
/// Adding a new `rel_type` here is the only path for engine gates to introduce one.
pub const ALLOWED_RELS: &[&str] = &[
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
    // Inter-entry semantic edges projected by upsert_relationships — supersedes
    // + blocks were written but never read (absent here => graph-query showed
    // `(no edges)`). See decision.cli-supersedes-projection-silent-fail.
    "supersedes",
    "blocks",
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

/// Bridge `GraphEntity` (properties as JSON text) to the public `Entity`
/// (properties as `serde_json::Value`).
fn entity_from_graph(e: GraphEntity) -> Entity {
    Entity {
        id: Some(e.id),
        entity_type: e.entity_type,
        name: e.name,
        properties: e.properties.and_then(|p| serde_json::from_str(&p).ok()),
        content_hash: e.content_hash,
        project: e.project,
    }
}

/// Find an entity by (`entity_type`, name); insert it if absent. Returns its id.
///
/// The id is a deterministic BLAKE3 hash of `(entity_type, name)` so re-ingest
/// of the same (type, name) converges to one row (idempotent upsert).
///
/// # Errors
/// `Error::Database` on SELECT or INSERT failure.
pub async fn upsert_entity(db: &DbStore, entity_type: &str, name: &str) -> Result<String> {
    if let Some(e) = db.find_entity(entity_type, name).await? {
        return Ok(e.id);
    }
    let id = crate::decisions::hash_keyed("entity", entity_type, name, "");
    db.upsert_entity(&GraphEntity::new(
        id.clone(),
        entity_type,
        name,
        None,
        None,
        None,
    ))
    .await?;
    Ok(id)
}

/// List entities, optionally filtered by `entity_type`. Capped at 5_000 rows.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn list_entities(db: &DbStore, entity_type: Option<&str>) -> Result<Vec<Entity>> {
    let rows = db.list_entities(entity_type).await?;
    Ok(rows.into_iter().map(entity_from_graph).collect())
}

/// Find an entity by (`entity_type`, name). Returns None if absent.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn find_entity(db: &DbStore, entity_type: &str, name: &str) -> Result<Option<Entity>> {
    Ok(db
        .find_entity(entity_type, name)
        .await?
        .map(entity_from_graph))
}

/// Add a relationship between two entities by string `rel_type`.
/// `rel_type` must be in `ALLOWED_RELS` — unknown names error to prevent typos.
///
/// # Errors
/// `Error::Migration` when `rel_type` is not in `ALLOWED_RELS`;
/// `Error::Database` from the INSERT.
pub async fn relate_dynamic(
    db: &DbStore,
    from: &str,
    to: &str,
    rel_type: &str,
    weight: f64,
) -> Result<()> {
    validate_rel(rel_type)?;
    db.relate_dynamic(from, to, rel_type, weight).await?;
    Ok(())
}

/// Relate a knowledge node to a citation (or a citation to a parent/dependency).
/// Rejects any edge not in `ALLOWED_CITATION_RELS` so a workflow/ontology edge
/// cannot leak into the citation DAG.
///
/// # Errors
/// `Error::Migration` when `rel_type` is not a citation edge; `Error::Database`
/// from the INSERT.
pub async fn relate_citation(
    db: &DbStore,
    from: &str,
    to: &str,
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

/// Single-query traversal: return the record ids of every node that cites
/// `citation` (incoming `cite` edges) — no N+1.
///
/// # Errors
/// Propagates `Error::Database` from the query.
pub async fn traverse_with_citations(db: &DbStore, citation: &str) -> Result<Vec<String>> {
    let edges = db.related_backward(citation).await?;
    Ok(edges
        .into_iter()
        .filter(|e| e.rel_type == "cite")
        .map(|e| e.from)
        .collect())
}

/// Forward complement of `traverse_with_citations`: return the citation record
/// ids that `node` cites (outgoing `cite` edges).
///
/// Lets a mistake/decision/roadmap node reach its merged citations without a
/// second query — the read half of the non-destructive C6 merge.
///
/// # Errors
/// Propagates `Error::Database` from the query.
pub async fn citations_cited_by(db: &DbStore, node: &str) -> Result<Vec<String>> {
    let edges = db.related_forward(node).await?;
    Ok(edges
        .into_iter()
        .filter(|e| e.rel_type == "cite")
        .map(|e| e.to)
        .collect())
}

/// One row of the related-entity result set: outgoing edge → target entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RelatedRow {
    pub rel_type: String,
    pub weight: f64,
    pub target: Entity,
}

/// One edge whose both endpoints fall inside a given node set. Endpoint ids are
/// stringified record ids matching the form the KG renderer keys nodes by.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EdgeRow {
    pub from: String,
    pub to: String,
    pub rel_type: String,
}

/// Return every allowed edge whose endpoints are both in `node_ids`.
///
/// These are the edges internal to the visible node set, so the graph view never
/// draws a line to an off-canvas node.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn list_edges_among(db: &DbStore, node_ids: &[String]) -> Result<Vec<EdgeRow>> {
    let edges = db.list_edges_among(node_ids).await?;
    Ok(edges
        .into_iter()
        .map(|e| EdgeRow {
            from: e.from,
            to: e.to,
            rel_type: e.rel_type,
        })
        .collect())
}

/// Return outgoing edges from `from` across all `ALLOWED_RELS`, paired with
/// the target entity. Limit caps the fan-out.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn get_related(db: &DbStore, from: &str, limit: usize) -> Result<Vec<RelatedRow>> {
    let rows = db.get_related(from, limit).await?;
    Ok(rows
        .into_iter()
        .map(|r| RelatedRow {
            rel_type: r.rel_type,
            weight: r.weight,
            target: entity_from_graph(r.target),
        })
        .collect())
}

#[cfg(test)]
#[path = "dynamic_citation_test.rs"]
mod citation_edge_tests;
