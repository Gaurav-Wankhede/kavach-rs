//! `[FLOW]` session-start read path — implementation-flow DAGs for the project.
//! Surfaces each stored flow as a rendered Mermaid `flowchart TD` so the model
//! sees the intended implementation order BEFORE it starts work (awareness).
use kavach_rpc::methods::db::{FlowRenderResult, FlowSummary};
use std::fmt::Write as _;

/// Max flows injected at session start (token discipline).
const FLOW_TOP_K: usize = 3;

/// Load project flows via `db.flow_list`, render each via `db.flow_render`, and
/// format an `[FLOW]` block for injection. Returns `None` when the project has
/// no flows or the daemon is unreachable (awareness is best-effort, never fatal).
#[must_use]
pub(super) fn flow_context(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let list_params = serde_json::json!({ "project_slug": project_slug });
    let flows: Vec<FlowSummary> =
        kavach_rpc::client::call("db.flow_list", Some(list_params)).ok()?;
    if flows.is_empty() {
        return None;
    }
    let mut ctx = String::from("\n[FLOW] implementation flows for this project (intended order)\n");
    let mut count = 0usize;
    for flow in flows.into_iter().take(FLOW_TOP_K) {
        let render_params = serde_json::json!({
            "project_slug": project_slug,
            "flow_key": flow.flow_key,
            "format": "mermaid",
        });
        let Ok(rendered): Result<FlowRenderResult, _> =
            kavach_rpc::client::call("db.flow_render", Some(render_params))
        else {
            continue;
        };
        if let Some(mermaid) = rendered.mermaid {
            writeln!(ctx, "• {} ({})", flow.flow_title, flow.flow_key).ok();
            writeln!(ctx, "```mermaid\n{}```", mermaid.trim_end()).ok();
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        return None;
    }
    ctx.push_str("apply: follow the DAG order; a step's prerequisites must land before it.\n");
    Some(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_returns_none() {
        assert!(flow_context("").is_none());
    }
}
