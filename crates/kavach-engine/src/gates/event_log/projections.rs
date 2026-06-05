//! Graph projections: turn logged events into entity + relationship edges via
//! the RPC primitives. All fire-and-forget; daemon-down silently no-ops.

use super::refs::{extract_content_references, memory_entry_qualified_name, skill_for_file};
use super::rpc::{log_raw_rpc, rpc_add_relationship, rpc_entity_upsert};

/// Project a `file_write` into graph edges via RPC.
pub(super) fn project_file_write_rpc(file_path: &str, project_slug: &str, content: &str) {
    let Some(file_id) = rpc_entity_upsert("file", file_path) else {
        return;
    };
    let skill = skill_for_file(file_path);
    if !skill.is_empty()
        && let Some(skill_id) = rpc_entity_upsert("skill", skill)
    {
        rpc_add_relationship(&file_id, &skill_id, "uses_skill", 1.0);
    }
    if !project_slug.is_empty()
        && let Some(proj_id) = rpc_entity_upsert("project", project_slug)
    {
        rpc_add_relationship(&file_id, &proj_id, "belongs_to", 1.0);
    }
    if !content.is_empty() {
        for target in extract_content_references(content) {
            if let Some(ref_id) = rpc_entity_upsert("skill", &target) {
                rpc_add_relationship(&file_id, &ref_id, "references", 1.0);
            }
        }
    }
}

/// Project a typed memory entry into the graph via kavach-rpc daemon.
///
/// Handles `roadmap/decision/pattern/research/app_spec`. Engine hot paths use
/// this; CLI one-shot writes use a direct-DB sibling. Fire-and-forget.
pub fn project_memory_entry_rpc(
    category: &str,
    entry_key: &str,
    project_slug: &str,
    content: &str,
) {
    let qualified_name = memory_entry_qualified_name(category, entry_key, project_slug);
    let Some(entry_id) = rpc_entity_upsert(category, &qualified_name) else {
        return;
    };
    if !project_slug.is_empty()
        && let Some(proj_id) = rpc_entity_upsert("project", project_slug)
    {
        rpc_add_relationship(&entry_id, &proj_id, "in_scope", 1.0);
    }
    if !content.is_empty() {
        for target in extract_content_references(content) {
            if let Some(ref_id) = rpc_entity_upsert("skill", &target) {
                rpc_add_relationship(&entry_id, &ref_id, "references", 1.0);
            }
        }
    }
}

/// Project a `session_start` into graph: session→project `works_on` edge.
pub(super) fn project_session_to_graph_rpc(session_id: &str, project_slug: &str) {
    let Some(sess_id) = rpc_entity_upsert("session", session_id) else {
        return;
    };
    let Some(proj_id) = rpc_entity_upsert("project", project_slug) else {
        return;
    };
    rpc_add_relationship(&sess_id, &proj_id, "works_on", 1.0);
}

/// Project a skill invocation: session→skill `uses_skill` edge.
pub(super) fn project_skill_invoke_to_graph_rpc(session_id: &str, skill_name: &str) {
    let Some(sess_id) = rpc_entity_upsert("session", session_id) else {
        return;
    };
    let Some(skill_id) = rpc_entity_upsert("skill", skill_name) else {
        return;
    };
    rpc_add_relationship(&sess_id, &skill_id, "uses_skill", 1.0);
}

/// Project a gate block as an audit event row.
pub(super) fn project_gate_block_to_event_rpc(
    session_id: &str,
    project_slug: &str,
    gate: &str,
    reason: &str,
) {
    let escaped = reason.replace('"', "'");
    let payload = format!(r#"{{"projection":"gate_block","gate":"{gate}","reason":"{escaped}"}}"#);
    log_raw_rpc(
        session_id,
        "gate_block_projection",
        "kavach-engine::projections",
        project_slug,
        Some(&payload),
    );
}
