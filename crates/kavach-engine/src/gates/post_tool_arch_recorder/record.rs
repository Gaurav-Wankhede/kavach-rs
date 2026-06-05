//! Architecture decision recording — RPC-backed (`SurrealDB` via kavach-rpc daemon).

use super::extract::extract_arch_comment;
use std::path::Path;

/// Record an architecture decision after Write/Edit.
/// Fire-and-forget via kavach-rpc — errors swallowed; gate must never fail.
/// `turn` is the harness turn counter; persisted on both the arch row and the
/// event row so audit queries can correlate ARCH choices with their turn.
pub(crate) fn record(file_path: &str, content: &str, project_slug: &str, turn: i64) {
    if !Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return;
    }
    let Some(arch) = extract_arch_comment(content) else {
        return;
    };

    // 1. Persist the arch_decision row via RPC.
    let arch_params = serde_json::json!({
        "project": project_slug,
        "pattern": arch.pattern,
        "scope": arch.scope,
        "cap_choice": arch.cap_choice,
        "failure_mode": arch.failure_mode,
        "tradeoff": arch.tradeoff,
        "file_path": file_path,
        "search_year": arch.search_year,
        "search_month": arch.search_month,
        "turn": turn,
    });
    kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "arch.upsert",
        Some(arch_params),
    )
    .ok();

    // 2. Append an event row.
    let payload = format!(
        r#"{{"pattern":"{}","scope":"{}","file":"{}","turn":{}}}"#,
        arch.pattern, arch.scope, file_path, turn
    );
    let event_params = serde_json::json!({
        "event_type": "architecture_decision",
        "source": "post_tool_arch_recorder",
        "project": project_slug,
        "payload": payload,
    });
    kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "event.append",
        Some(event_params),
    )
    .ok();

    // 3. Write graph edges file→pattern, pattern→scope.
    write_graph_edges_rpc(file_path, &arch.pattern, &arch.scope);
}

fn write_graph_edges_rpc(file_path: &str, pattern: &str, scope: &str) {
    let Some(file_id) = rpc_entity_upsert("file", file_path) else {
        return;
    };
    let Some(pattern_id) = rpc_entity_upsert("arch_pattern", pattern) else {
        return;
    };
    let Some(scope_id) = rpc_entity_upsert("arch_scope", scope) else {
        return;
    };
    rpc_add_relationship(&file_id, &pattern_id, "uses_pattern", 1.0);
    rpc_add_relationship(&pattern_id, &scope_id, "in_scope", 1.0);
}

fn rpc_entity_upsert(entity_type: &str, name: &str) -> Option<String> {
    let params = serde_json::json!({"entity_type": entity_type, "name": name});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call::<_, serde_json::Value>("graph.entity_upsert", Some(params));
    result
        .ok()
        .and_then(|v| v.get("id").and_then(|s| s.as_str()).map(ToOwned::to_owned))
}

fn rpc_add_relationship(from: &str, to: &str, rel_type: &str, weight: f64) {
    let params = serde_json::json!({
        "from": from, "to": to, "rel_type": rel_type, "weight": weight,
    });
    kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "graph.add_relationship",
        Some(params),
    )
    .ok();
}
