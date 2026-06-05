//! L3 stop-gate harness dispatch: when the claimed card carries a dynamic-
//! workflow harness, emit a `[AUTO_CONTINUE] run Workflow <path>` suffix so the
//! AI runs the compiled workflow instead of hand-executing the card. Reads the
//! card's harness link + the oracle's latest verdict via the sync RPC client
//! (`db.get_harness`, `db.latest_goal_attempt`). SOURCE: decision.goal-harness-6-patterns.
use serde_json::json;

/// Build the harness-dispatch suffix for a freshly-claimed card, or `None` when
/// the card has no harness (ordinary kanban dispatch). Fail-soft: any RPC error
/// or absent link yields `None` so the normal dispatch message still fires.
#[must_use]
pub(crate) fn harness_suffix(project: &str, key: &str) -> Option<String> {
    let link = kavach_rpc::client::call::<_, serde_json::Value>(
        "db.get_harness",
        Some(json!({ "project": project, "key": key })),
    )
    .ok()?;
    let harness = link.get("harness")?.as_str()?.to_owned();
    let workflow_path = link
        .get("workflow_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Some(format_suffix(
        &harness,
        &workflow_path,
        &latest_verdict(project),
    ))
}

/// Read the oracle's latest `goal_loop_attempt` verdict string, or "none" when
/// no attempt has run yet. Fail-soft to "unknown" on RPC error.
fn latest_verdict(project: &str) -> String {
    let Ok(v) = kavach_rpc::client::call::<_, serde_json::Value>(
        "db.latest_goal_attempt",
        Some(json!({ "project": project })),
    ) else {
        return "unknown".to_owned();
    };
    if v.get("found").and_then(serde_json::Value::as_bool) != Some(true) {
        return "none".to_owned();
    }
    v.get("payload")
        .and_then(|p| p.get("verdict"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("pending")
        .to_owned()
}

fn format_suffix(harness: &str, workflow_path: &str, verdict: &str) -> String {
    let run = if workflow_path.is_empty() {
        "compile the loop.yaml first (kavach goal compile), then run the emitted workflow.js"
            .to_owned()
    } else {
        format!("run Workflow {workflow_path}")
    };
    format!(
        " HARNESS [{harness}] (last verdict: {verdict}). [AUTO_CONTINUE] {run} \
         — the harness drives this card autonomously; do NOT hand-execute it."
    )
}

#[cfg(test)]
mod tests;
