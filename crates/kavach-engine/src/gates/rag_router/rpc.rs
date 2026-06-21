//! kavach-rpc wrappers for the RAG router. All fail-closed: an RPC error yields
//! `None`/empty/0 so the advisory gate degrades silently, never blocks.

/// RPC wrapper: find entity, return `RecordId` as "table:id" string.
pub(in crate::gates::rag_router) fn rpc_entity_find(
    entity_type: &str,
    name: &str,
) -> Option<String> {
    let params = serde_json::json!({"entity_type": entity_type, "name": name});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.entity_find", Some(params));
    let Ok(v) = result else {
        return None;
    };
    v.get("id").and_then(|id_val| {
        // RecordId serialized as {"tb": "entity", "id": {"String": "..."}}
        // or as plain string depending on SurrealDB version. Try string first.
        id_val.as_str().map(ToOwned::to_owned).or_else(|| {
            let tb = id_val.get("tb").and_then(|tb_val| tb_val.as_str())?;
            let key = id_val.get("id").and_then(|inner| {
                inner.as_str().map(ToOwned::to_owned).or_else(|| {
                    inner
                        .get("String")
                        .and_then(|s_val| s_val.as_str())
                        .map(ToOwned::to_owned)
                })
            })?;
            Some(format!("{tb}:{key}"))
        })
    })
}

/// RPC wrapper: upsert entity, return its `RecordId` as "table:id" string.
pub(in crate::gates::rag_router) fn rpc_entity_upsert(
    entity_type: &str,
    name: &str,
) -> Option<String> {
    let params = serde_json::json!({"entity_type": entity_type, "name": name});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.entity_upsert", Some(params));
    let Ok(v) = result else {
        return None;
    };
    v.get("id").and_then(|s| s.as_str()).map(ToOwned::to_owned)
}

/// RPC wrapper: add relationship between two entity IDs (fire-and-forget).
pub(in crate::gates::rag_router) fn rpc_add_relationship(
    from: &str,
    to: &str,
    rel_type: &str,
    weight: f64,
) {
    let params = serde_json::json!({
        "from": from, "to": to, "rel_type": rel_type, "weight": weight,
    });
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.add_relationship", Some(params));
    drop(result);
}

/// RPC wrapper: get `cross_invoke` neighbors of an entity.
pub(in crate::gates::rag_router) fn rpc_get_related_cross_invoke(from: &str) -> Vec<String> {
    let params = serde_json::json!({"from": from, "limit": 50});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.get_related", Some(params));
    let Ok(serde_json::Value::Array(arr)) = result else {
        return Vec::new();
    };
    arr.into_iter()
        .filter_map(|v| {
            let is_cross_invoke = v
                .get("rel_type")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "cross_invoke");
            if !is_cross_invoke {
                return None;
            }
            v.get("target")
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(ToOwned::to_owned)
        })
        .collect()
}

/// List all `rag_tree` labels via the kavach-rpc daemon. Empty on failure.
pub(in crate::gates::rag_router) fn all_labels() -> Vec<String> {
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call::<(), serde_json::Value>("rag.tree_list_labels", None);
    let Ok(serde_json::Value::Array(arr)) = result else {
        return Vec::new();
    };
    arr.into_iter()
        .filter_map(|v| {
            v.get("source")
                .and_then(|s| s.as_str())
                .map(ToOwned::to_owned)
        })
        .collect()
}
