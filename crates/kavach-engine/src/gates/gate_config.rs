//! Dynamic gate-config resolver façade — engine binding (`unit.gate-cfg-lift-resolver`).
//!
//! The resolver logic + wire type now live in `kavach-types` (a leaf crate the
//! pattern detectors can also reach). This module is the ENGINE binding: it
//! supplies the RPC transport (`kavach_rpc::client::call` against
//! `db.gate_config_get`, project-then-global) as the injected `call` closure,
//! and re-exports the typed resolvers so existing engine call sites
//! (`crate::gates::gate_config::gate_threshold(...)`) keep working unchanged.
//!
//! Fail-closed throughout: on any miss — no row, daemon down, malformed value —
//! the resolver returns the caller's compiled default (see `kavach_types`).
use kavach_types::GateValueDto;

/// The injected RPC transport: resolve `(project, gate_key)` via the daemon.
/// `.ok()` swallows transport/daemon-down (fail-closed to default); `flatten`
/// collapses a `Some(None)` "no row" to `None`.
fn rpc_fetch(project: &str, gate_key: &str) -> Option<GateValueDto> {
    let params = serde_json::json!({ "project": project, "gate_key": gate_key });
    kavach_rpc::client::call::<_, Option<GateValueDto>>("db.gate_config_get", Some(params))
        .ok()
        .flatten()
}

/// Resolve a numeric threshold, falling back to `default` on any miss.
#[must_use]
pub fn gate_threshold(project: &str, gate_key: &str, default: f64) -> f64 {
    kavach_types::gate_threshold(rpc_fetch, project, gate_key, default)
}

/// Resolve a boolean enablement toggle, falling back to `default` on any miss.
#[must_use]
pub fn gate_enabled(project: &str, gate_key: &str, default: bool) -> bool {
    kavach_types::gate_enabled(rpc_fetch, project, gate_key, default)
}

/// Resolve injected text (or a severity string), falling back to `default`.
#[must_use]
pub fn gate_text(project: &str, gate_key: &str, default: &str) -> String {
    kavach_types::gate_text(rpc_fetch, project, gate_key, default)
}

/// Resolve a detection-pattern / safelist ADDITIVELY (compiled floor + DB extras,
/// floor immutable). See `kavach_types::gate_patterns` for the invariant.
#[must_use]
pub fn gate_patterns(project: &str, gate_key: &str, default: &[&str]) -> Vec<String> {
    kavach_types::gate_patterns(rpc_fetch, project, gate_key, default)
}

#[cfg(test)]
#[path = "gate_config_test.rs"]
#[cfg(test)]
#[path = "gate_config_test.rs"]
mod tests;