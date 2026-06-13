//! Dynamic gate-config resolver façade (`unit.dynamic-gate-config-plane` P2).
//!
//! A gate that wants a runtime-tunable value calls one of the typed resolvers
//! here with its COMPILED DEFAULT as the fallback. The resolver asks the daemon
//! (`db.gate_config_get`, project-then-global) for an override; on ANY miss —
//! no row, daemon down, malformed value, wrong kind — it returns the compiled
//! default unchanged. This is the fail-closed contract: dynamic config can only
//! ever REPLACE a value the gate already had a safe default for; it can never
//! leave a gate with no value, and a dead daemon never changes gate behavior.
//!
//! Resolution chain realized here: DB (this call) > compiled default (the
//! `default` arg). The file layer (`kavach_config::GatesConfig`) is consulted by
//! the caller where it already does so; this façade adds only the DB overlay.
use kavach_rpc::methods::db::GateValueDto;

/// Fetch the raw override DTO for `(project, gate_key)`, or `None` on any miss
/// (absent row OR daemon unreachable OR parse failure). Never errors outward —
/// the whole point is that the caller proceeds with its default on `None`.
fn fetch(project: &str, gate_key: &str) -> Option<GateValueDto> {
    if project.is_empty() || gate_key.is_empty() {
        return None;
    }
    let params = serde_json::json!({ "project": project, "gate_key": gate_key });
    // `Option<GateValueDto>`: outer `.ok()?` swallows transport/daemon-down
    // (fail-closed to default); inner `flatten` collapses a `Some(None)` "no
    // row" to `None`.
    kavach_rpc::client::call::<_, Option<GateValueDto>>("db.gate_config_get", Some(params))
        .ok()
        .flatten()
}

/// Resolve a numeric threshold, falling back to `default` on any miss.
#[must_use]
pub fn gate_threshold(project: &str, gate_key: &str, default: f64) -> f64 {
    fetch(project, gate_key).and_then(|d| d.num).unwrap_or(default)
}

/// Resolve a boolean enablement toggle, falling back to `default` on any miss.
#[must_use]
pub fn gate_enabled(project: &str, gate_key: &str, default: bool) -> bool {
    fetch(project, gate_key)
        .and_then(|d| d.boolean)
        .unwrap_or(default)
}

/// Resolve injected text (or a severity string), falling back to `default`.
#[must_use]
pub fn gate_text(project: &str, gate_key: &str, default: &str) -> String {
    fetch(project, gate_key)
        .and_then(|d| d.text)
        .unwrap_or_else(|| default.to_owned())
}

/// Resolve a detection-pattern / safelist, ADDITIVELY: the returned list is the
/// compiled `default` floor with any DB-provided patterns appended. A DB row can
/// only ADD patterns — it can never remove a compiled one. This is the
/// security-gate fail-closed invariant (a P0 pattern is not deletable via DB).
#[must_use]
pub fn gate_patterns(project: &str, gate_key: &str, default: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = default.iter().map(|s| (*s).to_owned()).collect();
    if let Some(extra) = fetch(project, gate_key).and_then(|d| d.list) {
        for p in extra {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "gate_config_test.rs"]
mod tests;
