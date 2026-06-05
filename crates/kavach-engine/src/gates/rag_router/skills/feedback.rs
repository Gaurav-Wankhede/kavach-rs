//! Phase 3 feedback: write `session→uses_skill` edges for resolved skills.
//! Fire-and-forget via kavach-rpc — errors ignored to never block the gate.
use super::super::rpc::{rpc_add_relationship, rpc_entity_upsert};

/// Write `session→uses_skill` edges for each resolved skill.
pub(super) fn write_skill_feedback_edges(skills: &[String]) {
    if skills.is_empty() {
        return;
    }
    // Prefer env var; fall back to kavach session state (set by SessionStart gate).
    // CLAUDE_SESSION_ID is not reliably set by Claude Code in hook subprocesses.
    let session_id = std::env::var("CLAUDE_SESSION_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| kavach_session::get_or_create_session().session_id);
    if session_id.is_empty() {
        return;
    }
    let Some(sess_id_str) = rpc_entity_upsert("session", &session_id) else {
        return;
    };
    for skill in skills {
        if let Some(skill_id_str) = rpc_entity_upsert("skill", skill) {
            rpc_add_relationship(&sess_id_str, &skill_id_str, "uses_skill", 1.0);
        }
    }
}
