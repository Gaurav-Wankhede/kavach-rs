//! RPC primitives for event logging + graph projection. All go through
//! `kavach_rpc` to the SurrealDB-backed daemon; every call is fire-and-forget
//! so a gate never fails because the daemon is down.

/// Log an event row via kavach-rpc daemon (`SurrealDB`). Silently drops on
/// daemon-unavailable; callers must not depend on the row existing.
pub(super) fn log_raw_rpc(
    session_id: &str,
    event_type: &str,
    source: &str,
    project_slug: &str,
    payload: Option<&str>,
) {
    let sid = if session_id.is_empty() {
        ""
    } else {
        session_id
    };
    let payload_with_session = match payload {
        Some(p) if !sid.is_empty() => {
            let trimmed = p.trim_start_matches('{').trim_end_matches('}');
            Some(format!(r#"{{"sid":"{sid}",{trimmed}}}"#))
        }
        Some(p) => Some(p.to_owned()),
        None if !sid.is_empty() => Some(format!(r#"{{"sid":"{sid}"}}"#)),
        None => None,
    };
    let params = serde_json::json!({
        "event_type": event_type,
        "source": source,
        "project": if project_slug.is_empty() { None } else { Some(project_slug) },
        "payload": payload_with_session,
    });
    // INTENTIONAL: fire-and-forget — daemon may be down; gate must not block.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> = kavach_rpc::client::call("event.append", Some(params));
}

/// RPC wrapper: upsert entity, return its `RecordId` as "table:id" string.
pub(super) fn rpc_entity_upsert(entity_type: &str, name: &str) -> Option<String> {
    let params = serde_json::json!({"entity_type": entity_type, "name": name});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.entity_upsert", Some(params));
    result
        .ok()
        .and_then(|v| v.get("id").and_then(|s| s.as_str()).map(ToOwned::to_owned))
}

/// RPC wrapper: add relationship between two entity IDs (fire-and-forget).
pub(super) fn rpc_add_relationship(from: &str, to: &str, rel_type: &str, weight: f64) {
    let params = serde_json::json!({
        "from": from, "to": to, "rel_type": rel_type, "weight": weight,
    });
    // INTENTIONAL: fire-and-forget — graph edges are advisory.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; graph edges are advisory"
    )]
    let _: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.add_relationship", Some(params));
}
