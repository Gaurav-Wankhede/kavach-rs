use crate::error::{Error, Result};
use crate::graph::dynamic::relate_dynamic;
use crate::turso::TursoStore;
use crate::turso::graph::{GraphEntity, GraphStore};

use super::model::{FlowDag, FlowSpec, FlowStep};
use super::shape::NodeShape;

/// Resolve a project slug to its record id, or `Err(RecordNotFound)`.
async fn project_id(db: &TursoStore, project_slug: &str) -> Result<String> {
    let mut resp = db
        .exec_params(
            "SELECT slug FROM projects WHERE slug = $slug LIMIT 1",
            libsql::params![project_slug.to_owned()],
        )
        .await?;
    match resp.next().await? {
        Some(r) => Ok(r.get::<String>(0)?),
        None => Err(Error::RecordNotFound(format!(
            "project '{project_slug}' not registered"
        ))),
    }
}

/// Upsert an entity carrying `properties` and `project`, keyed on
/// (`entity_type`, `name`). Updates the row in place if it exists so re-ingest
/// of the same flow is idempotent. Returns the entity id.
async fn upsert_entity_props(
    db: &TursoStore,
    entity_type: &str,
    name: &str,
    project: &str,
    properties: serde_json::Value,
) -> Result<String> {
    let props_json = serde_json::to_string(&properties).map_err(Error::Json)?;
    if let Some(e) = db.find_entity(entity_type, name).await? {
        db.upsert_entity(&GraphEntity::new(
            e.id.clone(),
            entity_type,
            name,
            Some(props_json),
            e.content_hash,
            Some(project.to_owned()),
        ))
        .await?;
        return Ok(e.id);
    }
    let id = crate::decisions::hash_keyed("entity", entity_type, name, "");
    db.upsert_entity(&GraphEntity::new(
        id.clone(),
        entity_type,
        name,
        Some(props_json),
        None,
        Some(project.to_owned()),
    ))
    .await?;
    Ok(id)
}

/// Fully-qualified, globally-unique step entity name: `flow_step:{slug}/{flow}/{step}`.
fn step_name(project_slug: &str, flow_key: &str, step_id: &str) -> String {
    format!("{project_slug}/{flow_key}/{step_id}")
}

/// The flow anchor entity name: `{slug}/{flow_key}`.
fn flow_name(project_slug: &str, flow_key: &str) -> String {
    format!("{project_slug}/{flow_key}")
}

/// Upsert a flow: anchor + steps + `depends_on` edges. Rejects a cyclic flow
/// before writing edges (fail closed). Idempotent on re-ingest.
///
/// # Errors
/// - `Error::RecordNotFound` if the project is not registered.
/// - `Error::Migration` if the supplied edges form a cycle, or reference an
///   unknown `step_id`.
/// - `Error::TursoStore` on any underlying query failure.
pub async fn upsert_flow(db: &TursoStore, project_slug: &str, spec: &FlowSpec) -> Result<String> {
    let known: std::collections::HashSet<&str> =
        spec.steps.iter().map(|s| s.step_id.as_str()).collect();
    for e in &spec.edges {
        if !known.contains(e.from.as_str()) || !known.contains(e.to.as_str()) {
            return Err(Error::Migration(format!(
                "flow '{}' edge {}->{} references unknown step",
                spec.flow_key, e.from, e.to
            )));
        }
    }
    let probe = FlowDag {
        flow_key: spec.flow_key.clone(),
        flow_title: spec.flow_title.clone(),
        steps: spec
            .steps
            .iter()
            .map(|s| FlowStep {
                step_id: s.step_id.clone(),
                label: s.label.clone(),
                shape: s.shape.as_deref().map_or(NodeShape::Rect, NodeShape::parse),
                description: s.description.clone(),
            })
            .collect(),
        edges: spec
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect(),
        raw_mermaid: spec.raw_mermaid.clone(),
    };
    if let Some(cycle) = probe.detect_cycle() {
        return Err(Error::Migration(format!(
            "flow '{}' has a dependency cycle among steps: {}",
            spec.flow_key,
            cycle.join(", ")
        )));
    }

    let pid = project_id(db, project_slug).await?;

    let anchor_props = serde_json::json!({
        "flow_key": spec.flow_key,
        "flow_title": spec.flow_title,
        "raw_mermaid": spec.raw_mermaid,
    });
    let anchor_id = upsert_entity_props(
        db,
        "flow",
        &flow_name(project_slug, &spec.flow_key),
        &pid,
        anchor_props,
    )
    .await?;

    // Remove the previous step set: delete the anchor's `contains` edges and
    // every `depends_on` edge among its steps, then re-create below. The old
    // SurrealQL `LET $steps = (SELECT VALUE out FROM $anchor->contains)` +
    // `DELETE depends_on WHERE in IN $steps` is a two-statement sweep; SQL
    // does it with two DELETE statements keyed on the anchor id.
    db.exec_params(
        "DELETE FROM entity_edges WHERE from_id = $anchor AND rel_type = 'contains'",
        libsql::params![anchor_id.clone()],
    )
    .await?;
    db.exec_params(
        "DELETE FROM entity_edges WHERE rel_type = 'depends_on' AND from_id IN \
           (SELECT to_id FROM entity_edges WHERE from_id = $anchor AND rel_type = 'contains')",
        libsql::params![anchor_id.clone()],
    )
    .await?;

    let mut step_ids: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    for s in &spec.steps {
        let shape = s.shape.as_deref().map_or(NodeShape::Rect, NodeShape::parse);
        let props = serde_json::json!({
            "flow_key": spec.flow_key,
            "step_id": s.step_id,
            "label": s.label,
            "shape": shape.as_str(),
            "description": s.description,
        });
        let id = upsert_entity_props(
            db,
            "flow_step",
            &step_name(project_slug, &spec.flow_key, &s.step_id),
            &pid,
            props,
        )
        .await?;
        relate_dynamic(db, &anchor_id, &id, "contains", 1.0).await?;
        step_ids.insert(s.step_id.as_str(), id);
    }

    for e in &spec.edges {
        if let (Some(from), Some(to)) = (step_ids.get(e.from.as_str()), step_ids.get(e.to.as_str()))
        {
            relate_dynamic(db, from, to, "depends_on", 1.0).await?;
        }
    }

    Ok(anchor_id)
}
