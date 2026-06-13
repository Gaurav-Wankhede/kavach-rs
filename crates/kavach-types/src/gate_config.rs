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
    fetch(call, project, gate_key)
        .and_then(|d| d.num)
        .unwrap_or(default)
}

/// Resolve a boolean enablement toggle, falling back to `default` on any miss.
pub fn gate_enabled<F>(call: F, project: &str, gate_key: &str, default: bool) -> bool
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    fetch(call, project, gate_key)
        .and_then(|d| d.boolean)
        .unwrap_or(default)
}

/// Resolve injected text (or a severity string), falling back to `default`.
pub fn gate_text<F>(call: F, project: &str, gate_key: &str, default: &str) -> String
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    fetch(call, project, gate_key)
        .and_then(|d| d.text)
        .unwrap_or_else(|| default.to_owned())
}

/// Resolve a detection-pattern / safelist, ADDITIVELY.
///
/// The returned list is the compiled `default` floor with any DB-provided
/// patterns appended. A DB row can only ADD patterns — it can never remove a
/// compiled one. This is the security-gate fail-closed invariant (a P0 pattern
/// is not deletable via DB).
pub fn gate_patterns<F>(call: F, project: &str, gate_key: &str, default: &[&str]) -> Vec<String>
where
    F: FnOnce(&str, &str) -> Option<GateValueDto>,
{
    let mut out: Vec<String> = default.iter().map(|s| (*s).to_owned()).collect();
    if let Some(extra) = fetch(call, project, gate_key).and_then(|d| d.list) {
        for p in extra {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A miss (call returns None) must yield the compiled default, never panic.
    #[test]
    fn miss_falls_back_to_default() {
        let miss = |_: &str, _: &str| None;
        assert!((gate_threshold(miss, "p", "k", 0.85) - 0.85).abs() < f64::EPSILON);
        assert!(gate_enabled(miss, "p", "k", true));
        assert_eq!(gate_text(miss, "p", "k", "D"), "D");
    }

    /// Empty project/key short-circuits to a miss before the call fires.
    #[test]
    fn empty_inputs_short_circuit() {
        let boom = |_: &str, _: &str| -> Option<GateValueDto> { panic!("call must not fire") };
        assert!((gate_threshold(boom, "", "k", 1.0) - 1.0).abs() < f64::EPSILON);
    }

    /// A hit replaces the default for the matching field.
    #[test]
    fn hit_replaces_default() {
        let hit = |_: &str, _: &str| {
            Some(GateValueDto {
                kind: "threshold".to_owned(),
                num: Some(0.42),
                boolean: None,
                list: None,
                text: None,
            })
        };
        assert!((gate_threshold(hit, "p", "k", 0.85) - 0.42).abs() < f64::EPSILON);
    }

    /// Patterns are additive: floor preserved, DB extras appended, no dup.
    #[test]
    fn patterns_are_additive_floor_immutable() {
        let extra = |_: &str, _: &str| {
            Some(GateValueDto {
                kind: "pattern_list".to_owned(),
                num: None,
                boolean: None,
                list: Some(vec!["rm -rf".to_owned(), "custom".to_owned()]),
                text: None,
            })
        };
        let floor = ["rm -rf", "DROP TABLE"];
        let resolved = gate_patterns(extra, "p", "k", &floor);
        // Floor entries always present; the duplicate "rm -rf" is not re-added;
        // the new "custom" is appended.
        assert_eq!(
            resolved,
            vec![
                "rm -rf".to_owned(),
                "DROP TABLE".to_owned(),
                "custom".to_owned()
            ]
        );
    }
}
