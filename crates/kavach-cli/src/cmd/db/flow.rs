// `kavach db flow-add` / `flow-show` — implementation-flow DAG ingest + render.
// Ingest is STRUCTURED JSON (steps + edges) so there is no Mermaid parser to
// maintain; the DAG is the store and Mermaid is rendered on read by the daemon.
use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_surreal::{FlowEdgeInput, FlowStepInput};
use std::io::Read as _;

/// JSON shape accepted on `--steps-json` (file path) or stdin.
#[derive(serde::Deserialize)]
struct FlowJson {
    steps: Vec<StepJson>,
    #[serde(default)]
    edges: Vec<EdgeJson>,
}

#[derive(serde::Deserialize)]
struct StepJson {
    // Documented JSON key is `id` (see flow-add --help + global CLAUDE.md/Cursor
    // rules); `step_id` kept as an alias so older payloads still parse. Without
    // this rename the documented `{"id":...}` shape failed to deserialize — a
    // doc-vs-impl mismatch.
    #[serde(rename = "id", alias = "step_id")]
    step_id: String,
    label: String,
    #[serde(default)]
    shape: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct EdgeJson {
    from: String,
    to: String,
}

fn emit_err(msg: &str) -> i32 {
    if let Err(io_err) = ewrite_or_exit(msg) {
        return into_exit_code(io_err);
    }
    1
}

/// Read the flow JSON from `path` (when `Some`) or stdin.
fn read_flow_json(path: Option<&str>) -> Result<FlowJson, String> {
    let raw = if let Some(p) = path {
        std::fs::read_to_string(p).map_err(|e| format!("read {p}: {e}"))?
    } else {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("read stdin: {e}"))?;
        buf
    };
    serde_json::from_str(&raw).map_err(|e| format!("parse flow JSON: {e}"))
}

/// `kavach db flow-add` — ingest a structured flow and persist its DAG.
pub(crate) fn add(
    project: &str,
    key: &str,
    title: &str,
    steps_json: Option<&str>,
    mermaid: Option<&str>,
) -> i32 {
    let parsed = match read_flow_json(steps_json) {
        Ok(p) => p,
        Err(e) => return emit_err(&format!("flow-add: {e}")),
    };
    let steps: Vec<FlowStepInput> = parsed
        .steps
        .into_iter()
        .map(|s| FlowStepInput {
            step_id: s.step_id,
            label: s.label,
            shape: s.shape,
            description: s.description,
        })
        .collect();
    let edges: Vec<FlowEdgeInput> = parsed
        .edges
        .into_iter()
        .map(|e| FlowEdgeInput {
            from: e.from,
            to: e.to,
        })
        .collect();
    let raw_mermaid = mermaid.map(ToOwned::to_owned);
    match rpc_client::flow_upsert(project, key, title, steps, edges, raw_mermaid) {
        Ok(r) => {
            let msg = format!(
                "flow '{key}' stored: {} step(s) [{}]",
                r.step_count, r.flow_id
            );
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            0
        }
        Err(e) => emit_err(&format!("flow-add: {e}")),
    }
}

/// `kavach db flow-show` — render a stored flow as Mermaid (default) or JSON.
pub(crate) fn show(project: &str, key: &str, format: &str) -> i32 {
    match rpc_client::flow_render(project, key, format) {
        Ok(r) => {
            let body = r.mermaid.or_else(|| {
                r.dag
                    .as_ref()
                    .and_then(|d| serde_json::to_string_pretty(d).ok())
            });
            match body {
                Some(text) => {
                    if let Err(io_err) = print_or_exit(&text) {
                        return into_exit_code(io_err);
                    }
                    0
                }
                None => emit_err("flow-show: empty render result"),
            }
        }
        Err(e) => emit_err(&format!("flow-show: {e}")),
    }
}
