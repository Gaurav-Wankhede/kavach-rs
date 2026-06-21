// Fire-and-forget bump of bulk_manifest conformance counters.
// SOURCE: decision.engine.bulk_mode_ttl_bounds_liability.

/// Increment `manifest.conformance_applied` for an in-flight bulk sweep.
pub(crate) fn emit_apply(sweep_id: &str) {
    let params = serde_json::json!({ "sweep_id": sweep_id, "field": "applied" });
    // INTENTIONAL: fire-and-forget — manifest TTL bounds liability if daemon down.
    kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "bulk.sweep_apply_event",
        Some(params),
    )
    .ok();
}
