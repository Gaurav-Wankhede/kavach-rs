// Fire-and-forget bump of bulk_manifest conformance counters.
// Called from post_write when KAVACH_BULK_SWEEP_ID env is set. Daemon down
// means the manifest's TTL still bounds liability — the audit row remains
// + per-edit event_log line still lands. SOURCE: roadmap.unit.kavach-bulk-mode.
// Phase 5 (stop-gate) will add emit_refused/emit_drifted when those branches
// land; per dead-code policy we only ship symbols with current call sites.

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
