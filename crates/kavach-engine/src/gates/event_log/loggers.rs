//! Public gate-facing loggers: each records an event row and fires the relevant
//! graph projection. Consumed by the intent, `post_write`, session, `pre_write`,
//! skill, and `post_tool_failure` gates.

use super::projections::{
    project_file_write_rpc, project_gate_block_to_event_rpc, project_session_to_graph_rpc,
    project_skill_invoke_to_graph_rpc,
};
use super::rpc::log_raw_rpc;

/// Log a file write event and project graph edges.
pub(crate) fn log_file_write(
    session_id: &str,
    file_path: &str,
    tool: &str,
    project_slug: &str,
    content: &str,
) {
    let payload = format!(r#""file":"{file_path}","tool":"{tool}""#);
    log_raw_rpc(
        session_id,
        "file_write",
        "gate:post_write",
        project_slug,
        Some(&format!("{{{payload}}}")),
    );
    project_file_write_rpc(file_path, project_slug, content);
}

/// Log an intent classification.
pub(crate) fn log_intent(session_id: &str, intent_type: &str, risk: &str, project_slug: &str) {
    let payload = format!(r#"{{"intent":"{intent_type}","risk":"{risk}"}}"#);
    log_raw_rpc(
        session_id,
        "intent_classified",
        "gate:intent",
        project_slug,
        Some(&payload),
    );
}

/// Log a session lifecycle event; create session→project edge on `session_start`.
pub(crate) fn log_session(session_id: &str, event_type: &str, model: &str, project_slug: &str) {
    let payload = format!(r#"{{"model":"{model}"}}"#);
    log_raw_rpc(
        session_id,
        event_type,
        "gate:session",
        project_slug,
        Some(&payload),
    );
    if event_type == "session_start" && !project_slug.is_empty() && !session_id.is_empty() {
        project_session_to_graph_rpc(session_id, project_slug);
    }
}

/// Log a gate decision and project block reasons as audit events.
pub(crate) fn log_gate_decision(
    session_id: &str,
    gate: &str,
    decision: &str,
    reason: &str,
    project_slug: &str,
) {
    let escaped = reason.replace('"', "'");
    let payload = format!(r#"{{"gate":"{gate}","decision":"{decision}","reason":"{escaped}"}}"#);
    log_raw_rpc(
        session_id,
        "gate_decision",
        gate,
        project_slug,
        Some(&payload),
    );
    if decision == "block" {
        project_gate_block_to_event_rpc(session_id, project_slug, gate, reason);
    }
}

/// Log a skill invocation for dynamic loadout scoring.
pub(crate) fn log_skill_invoke(session_id: &str, skill_name: &str, project_slug: &str) {
    let payload = format!(r#"{{"skill":"{skill_name}"}}"#);
    log_raw_rpc(
        session_id,
        "skill_invoke",
        "gate:skill",
        project_slug,
        Some(&payload),
    );
    if !session_id.is_empty() && !skill_name.is_empty() {
        project_skill_invoke_to_graph_rpc(session_id, skill_name);
    }
}

/// Inputs to `log_tool_failure`. Grouped to satisfy `clippy::too_many_arguments`.
pub(crate) struct ToolFailureLog<'a> {
    pub session_id: &'a str,
    pub tool_name: &'a str,
    pub error: &'a str,
    pub fix_strategy: &'a str,
    pub imperative_rewrite: &'a str,
    pub dsa_rationale: &'a str,
    pub gate_name: &'a str,
    pub project_slug: &'a str,
}

/// Log a tool failure and seed the self-evolution pattern store via
/// `gate_pattern.upsert` RPC.
pub(crate) fn log_tool_failure(input: &ToolFailureLog<'_>) {
    let escaped_error = input.error.replace('"', "'");
    let payload = format!(
        r#"{{"tool":"{}","error":"{escaped_error}","gate":"{}"}}"#,
        input.tool_name, input.gate_name
    );
    log_raw_rpc(
        input.session_id,
        "tool_failure",
        input.gate_name,
        input.project_slug,
        Some(&payload),
    );
    if !input.error.is_empty() && !input.project_slug.is_empty() {
        let params = serde_json::json!({
            "project": input.project_slug,
            "error_tokens": input.error,
            "fix_strategy": input.fix_strategy,
            "imperative_rewrite": input.imperative_rewrite,
            "dsa_rationale": input.dsa_rationale,
            "tool_name": input.tool_name,
            "gate_name": input.gate_name,
        });
        // Fire-and-forget but NON-LOSSY: daemon-down is spooled + replayed next
        // Stop, so an advisory gate-pattern signal is never lost.
        crate::gates::stop::spool_writes::call_or_spool("gate_pattern.upsert", params);
    }
}
