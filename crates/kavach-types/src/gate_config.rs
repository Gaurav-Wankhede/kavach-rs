//! Shared gate-config types + resolver facade (`unit.gate-cfg-lift-resolver`).
//!
//! Lives in `kavach-types` (a pure leaf crate) so BOTH `kavach-engine` (mid-tree)
//! and `kavach-patterns` (a leaf below `kavach-rpc`) can resolve a dynamic gate
//! value. The RPC transport is INJECTED as a `call` closure rather than depended
//! on directly — that keeps this crate transport-free and breaks the cycle that
//! would arise if it imported `kavach-rpc` (which itself depends on
//! `kavach-patterns`).
//!
//! Resolution chain realized here: DB overlay (via the injected `call`) >
//! compiled default (the `default` arg). On ANY miss — no row, daemon down,
//! malformed value, wrong kind — the resolver returns the compiled default
//! unchanged (fail-closed: dynamic config can only REPLACE a value a gate
//! already had a safe default for; it never leaves a gate value-less).
use serde::{Deserialize, Serialize};

/// The global (project-agnostic) row sentinel.
///
/// Mirrors `kavach-database`'s `GLOBAL_PROJECT`. A gate with no project in scope
/// (e.g. a pre-write content scanner) resolves its overlay under this key.
pub const GLOBAL_PROJECT_KEY: &str = "*";

/// Wire shape for a gate-config value: a kind tag plus one populated value field.
///
/// Flat (not a Rust enum) so it serializes cleanly over JSON-RPC and is trivial
/// to construct from any client. The canonical definition — `kavach-rpc`
/// re-exports this rather than redefining it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "stable wire DTO constructed at every RPC handler + client boundary"
)]
pub struct GateValueDto {
    /// `threshold` | `pattern_list` | `enabled` | `severity` | `text`.
    pub kind: String,
    #[serde(default)]
    pub num: Option<f64>,
    #[serde(default)]
    pub boolean: Option<bool>,
    #[serde(default)]
    pub list: Option<Vec<String>>,
    #[serde(default)]
    pub text: Option<String>,
}

/// Fetch the raw override DTO via the injected `call`, or `None` on any miss.
///
/// A miss is empty inputs, absent row, or daemon unreachable. The `call` closure
/// owns the transport: it is handed the `(project, gate_key)` and returns the
/// resolved DTO or `None`. Never panics — the caller proceeds with its default.
fn fetch<F>(call: F, project: &str, gate_key: &str) -> Option<GateValueDto>
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    if project.is_empty() || gate_key.is_empty() {
        return None;
    }
    call(project, gate_key)
}

/// Resolve a numeric threshold, falling back to `default` on any miss.
pub fn gate_threshold<F>(call: F, project: &str, gate_key: &str, default: f64) -> f64
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    match fetch(call, project, gate_key) {
        Some(GateValueDto { num: Some(v), .. }) => v,
        Some(_) | None => default,
    }
}

/// Resolve a boolean enablement toggle, falling back to `default` on any miss.
pub fn gate_enabled<F>(call: F, project: &str, gate_key: &str, default: bool) -> bool
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    match fetch(call, project, gate_key) {
        Some(GateValueDto {
            boolean: Some(v), ..
        }) => v,
        Some(_) | None => default,
    }
}

/// Resolve injected text (or a severity string), falling back to `default`.
pub fn gate_text<F>(call: F, project: &str, gate_key: &str, default: &str) -> String
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    match fetch(call, project, gate_key) {
        Some(GateValueDto { text: Some(v), .. }) => v,
        Some(_) | None => default.to_owned(),
    }
}

/// Resolve a detection-pattern / safelist, ADDITIVELY.
pub fn gate_patterns<F>(call: F, project: &str, gate_key: &str, default: &[&str]) -> Vec<String>
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    let mut seen: std::collections::HashSet<String> =
        default.iter().map(|s| (*s).to_owned()).collect();
    let mut out: Vec<String> = default.iter().map(|s| (*s).to_owned()).collect();
    if let Some(extra) = fetch(call, project, gate_key).and_then(|d| d.list) {
        for p in extra {
            if seen.insert(p.clone()) {
                out.push(p);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "gate_config_test.rs"]
mod tests;
