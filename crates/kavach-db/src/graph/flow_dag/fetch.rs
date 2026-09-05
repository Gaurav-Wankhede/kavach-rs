use crate::error::{Error, Result};
use crate::turso::TursoStore;

use super::model::{FlowDag, FlowStep};
use super::shape::NodeShape;

fn prop_str(props: Option<&serde_json::Value>, key: &str) -> Option<String> {
    props
        .and_then(|v| v.get(key))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn props_from_row(r: &libsql::Row, idx: i32) -> Option<serde_json::Value> {
    r.get::<Option<String>>(idx)
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// Fetch a flow's DAG (anchor metadata + steps + `depends_on` edges).
///
/// # Errors
/// - `Error::RecordNotFound` if the project or the flow does not exist.
/// - `Error::Turso` on query failure.
pub async fn fetch_flow(db: &TursoStore, project_slug: &str, flow_key: &str) -> Result<FlowDag> {
    let anchor_name = format!("{project_slug}/{flow_key}");
    let mut aresp = db
        .exec_params(
            "SELECT properties FROM entities \
             WHERE entity_type = 'flow' AND name = $name LIMIT 1",
            libsql::params![anchor_name],
        )
        .await?;
    let anchor_props = match aresp.next().await? {
        Some(r) => props_from_row(&r, 0),
        None => {
            return Err(Error::RecordNotFound(format!(
                "flow '{flow_key}' not found in project '{project_slug}'"
            )));
        }
    };

    let prefix = format!("{project_slug}/{flow_key}/");
    let mut sresp = db
        .exec_params(
            "SELECT name, properties FROM entities \
             WHERE entity_type = 'flow_step' AND name LIKE $prefix",
            libsql::params![format!("{prefix}%")],
        )
        .await?;
    let mut steps: Vec<FlowStep> = Vec::new();
    let mut step_names: Vec<String> = Vec::new();
    while let Some(r) = sresp.next().await? {
        let name = r.get::<String>(0)?;
        let props = props_from_row(&r, 1);
        let step_id = prop_str(props.as_ref(), "step_id")
            .unwrap_or_else(|| name.rsplit('/').next().unwrap_or(name.as_str()).to_owned());
        let label = prop_str(props.as_ref(), "label").unwrap_or_else(|| step_id.clone());
        let shape =
            prop_str(props.as_ref(), "shape").map_or(NodeShape::Rect, |s| NodeShape::parse(&s));
        let description = prop_str(props.as_ref(), "description");
        steps.push(FlowStep {
            step_id,
            label,
            shape,
            description,
        });
        step_names.push(name);
    }

    // depends_on edges among the flow's own steps. Edges store entity IDs
    // (content hashes); join entities to resolve the qualified names so the
    // step_id extraction below sees `p/auth/validate`, not a hash.
    let mut edges: Vec<(String, String)> = Vec::new();
    if !step_names.is_empty() {
        let placeholders = step_names
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let eq = format!(
            "SELECT f.name, t.name FROM entity_edges e \
             JOIN entities f ON f.id = e.from_id \
             JOIN entities t ON t.id = e.to_id \
             WHERE e.rel_type = 'depends_on' AND f.name IN ({placeholders})"
        );
        let mut eresp = db.exec_params(&eq, step_names.clone()).await?;
        while let Some(r) = eresp.next().await? {
            let from = r.get::<String>(0)?;
            let to = r.get::<String>(1)?;
            let from_step = from.rsplit('/').next().unwrap_or(&from).to_owned();
            let to_step = to.rsplit('/').next().unwrap_or(&to).to_owned();
            edges.push((from_step, to_step));
        }
    }

    Ok(FlowDag {
        flow_key: flow_key.to_owned(),
        flow_title: prop_str(anchor_props.as_ref(), "flow_title")
            .unwrap_or_else(|| flow_key.to_owned()),
        steps,
        edges,
        raw_mermaid: prop_str(anchor_props.as_ref(), "raw_mermaid"),
    })
}

/// List the flow keys + titles defined for a project (for awareness injection).
///
/// # Errors
/// `Error::Turso` on query failure.
pub async fn list_flows(db: &TursoStore, project_slug: &str) -> Result<Vec<(String, String)>> {
    let prefix = format!("{project_slug}/");
    let mut resp = db
        .exec_params(
            "SELECT properties FROM entities \
             WHERE entity_type = 'flow' AND name LIKE $prefix",
            libsql::params![format!("{prefix}%")],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = resp.next().await? {
        let props = props_from_row(&r, 0);
        if let Some(key) = prop_str(props.as_ref(), "flow_key") {
            let title = prop_str(props.as_ref(), "flow_title").unwrap_or_else(|| key.clone());
            out.push((key, title));
        }
    }
    Ok(out)
}
