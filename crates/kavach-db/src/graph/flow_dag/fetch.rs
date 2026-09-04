use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

use super::model::{FlowDag, FlowStep};
use super::shape::NodeShape;

#[derive(SurrealValue)]
pub(super) struct AnchorRow {
    properties: Option<serde_json::Value>,
}

#[derive(SurrealValue)]
pub(super) struct StepRow {
    name: String,
    properties: Option<serde_json::Value>,
}

#[derive(SurrealValue)]
pub(super) struct FlowEdgeRow {
    from_step: Option<String>,
    to_steps: Vec<String>,
}

fn prop_str(props: Option<&serde_json::Value>, key: &str) -> Option<String> {
    props
        .and_then(|v| v.get(key))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

/// Fetch a flow's DAG (anchor metadata + steps + `depends_on` edges).
///
/// # Errors
/// - `Error::RecordNotFound` if the project or the flow does not exist.
/// - `Error::Surreal` on query failure.
pub async fn fetch_flow(db: &Surreal<Db>, project_slug: &str, flow_key: &str) -> Result<FlowDag> {
    let anchor_name = format!("{project_slug}/{flow_key}");
    let aq = "SELECT properties FROM entity \
              WHERE entity_type = 'flow' AND name = $name LIMIT 1";
    let mut aresp = db.query(aq).bind(("name", anchor_name.clone())).await?;
    let anchor: Option<AnchorRow> = aresp.take(0)?;
    let Some(anchor) = anchor else {
        return Err(Error::RecordNotFound(format!(
            "flow '{flow_key}' not found in project '{project_slug}'"
        )));
    };

    let prefix = format!("{project_slug}/{flow_key}/");
    let sq = "SELECT name, properties FROM entity \
              WHERE entity_type = 'flow_step' AND string::starts_with(name, $prefix)";
    let mut sresp = db.query(sq).bind(("prefix", prefix.clone())).await?;
    let rows: Vec<StepRow> = sresp.take(0)?;

    let mut steps: Vec<FlowStep> = Vec::with_capacity(rows.len());
    let mut edges: Vec<(String, String)> = Vec::new();
    for row in &rows {
        let step_id = prop_str(row.properties.as_ref(), "step_id").unwrap_or_else(|| {
            row.name
                .rsplit('/')
                .next()
                .unwrap_or(row.name.as_str())
                .to_owned()
        });
        let label = prop_str(row.properties.as_ref(), "label").unwrap_or_else(|| step_id.clone());
        let shape = prop_str(row.properties.as_ref(), "shape")
            .map_or(NodeShape::Rect, |s| NodeShape::parse(&s));
        let description = prop_str(row.properties.as_ref(), "description");
        steps.push(FlowStep {
            step_id,
            label,
            shape,
            description,
        });
    }

    let eq = "SELECT properties.step_id AS from_step, \
              ->depends_on->entity.properties.step_id AS to_steps \
              FROM entity \
              WHERE entity_type = 'flow_step' AND string::starts_with(name, $prefix)";
    let mut eresp = db.query(eq).bind(("prefix", prefix.clone())).await?;
    let erows: Vec<FlowEdgeRow> = eresp.take(0)?;
    for er in erows {
        if let Some(from) = er.from_step {
            for to in er.to_steps {
                edges.push((from.clone(), to));
            }
        }
    }

    Ok(FlowDag {
        flow_key: flow_key.to_owned(),
        flow_title: prop_str(anchor.properties.as_ref(), "flow_title")
            .unwrap_or_else(|| flow_key.to_owned()),
        steps,
        edges,
        raw_mermaid: prop_str(anchor.properties.as_ref(), "raw_mermaid"),
    })
}

/// List the flow keys + titles defined for a project (for awareness injection).
///
/// # Errors
/// `Error::Surreal` on query failure.
pub async fn list_flows(db: &Surreal<Db>, project_slug: &str) -> Result<Vec<(String, String)>> {
    let prefix = format!("{project_slug}/");
    let q = "SELECT properties FROM entity \
             WHERE entity_type = 'flow' AND string::starts_with(name, $prefix)";
    let mut resp = db.query(q).bind(("prefix", prefix)).await?;
    let rows: Vec<AnchorRow> = resp.take(0)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(key) = prop_str(row.properties.as_ref(), "flow_key") {
            let title =
                prop_str(row.properties.as_ref(), "flow_title").unwrap_or_else(|| key.clone());
            out.push((key, title));
        }
    }
    Ok(out)
}
