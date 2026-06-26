use crate::error::{Error, Result};
use crate::graph::dynamic::relate_dynamic;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;
use surrealdb_types::SurrealValue;

use super::model::{FlowDag, FlowSpec, FlowStep};
use super::shape::NodeShape;

#[derive(SurrealValue)]
pub(super) struct EntityIdRow {
    id: RecordId,
}

#[derive(SurrealValue)]
pub(super) struct ProjectIdRow {
    id: RecordId,
}

/// Resolve a project slug to its record id, or `Err(RecordNotFound)`.
async fn project_id(db: &Surreal<Db>, project_slug: &str) -> Result<RecordId> {
    let q = "SELECT id FROM project WHERE slug = $slug LIMIT 1";
    let mut resp = db.query(q).bind(("slug", project_slug.to_owned())).await?;
    let row: Option<ProjectIdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("project '{project_slug}' not registered")))
}

/// Upsert an entity carrying `properties` and `project`, keyed on
/// (`entity_type`, `name`). Updates the row in place if it exists so re-ingest
/// of the same flow is idempotent. Returns the entity id.
async fn upsert_entity_props(
    db: &Surreal<Db>,
    entity_type: &str,
    name: &str,
    project: &RecordId,
    properties: serde_json::Value,
) -> Result<RecordId> {
    let find_q = "SELECT id FROM entity WHERE entity_type = $type AND name = $name LIMIT 1";
    let mut resp = db
        .query(find_q)
        .bind(("type", entity_type.to_owned()))
        .bind(("name", name.to_owned()))
        .await?;
    let existing: Option<EntityIdRow> = resp.take(0)?;
    if let Some(row) = existing {
        let upd = "UPDATE $id SET properties = $props, project = $project RETURN id";
        let mut r = db
            .query(upd)
            .bind(("id", row.id.clone()))
            .bind(("props", properties))
            .bind(("project", project.clone()))
            .await?;
        let updated: Option<EntityIdRow> = r.take(0)?;
        return Ok(updated.map_or(row.id, |u| u.id));
    }
    let create_q = "CREATE entity SET entity_type = $type, name = $name, \
                    properties = $props, project = $project RETURN id";
    let mut cresp = db
        .query(create_q)
        .bind(("type", entity_type.to_owned()))
        .bind(("name", name.to_owned()))
        .bind(("props", properties))
        .bind(("project", project.clone()))
        .await?;
    let row: Option<EntityIdRow> = cresp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound("flow entity create returned no id".into()))
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
/// - `Error::Surreal` on any underlying query failure.
pub async fn upsert_flow(
    db: &Surreal<Db>,
    project_slug: &str,
    spec: &FlowSpec,
) -> Result<RecordId> {
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

    db.query(
        "LET $steps = (SELECT VALUE out FROM $anchor->contains); \
         DELETE depends_on WHERE in IN $steps; \
         DELETE contains WHERE in = $anchor;",
    )
    .bind(("anchor", anchor_id.clone()))
    .await?;

    let mut step_ids: std::collections::HashMap<&str, RecordId> = std::collections::HashMap::new();
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
