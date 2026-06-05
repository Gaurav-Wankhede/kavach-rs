//! Self-evolve context injection: hot gate-patterns + the mistake ledger.
//! The K-PRI-ranked mistake ledger lives in the `mistakes` submodule.
mod mistakes;

use std::fmt::Write as _;

pub(super) use mistakes::mistake_ledger_context;

/// Load top autonomous gate patterns for the project and format as context.
/// Injected at session start so Claude immediately knows cached fixes without
/// waiting for a tool failure to trigger the Tier 1 lookup path.
/// Returns None if the DB is unavailable or no autonomous patterns exist.
pub(super) fn hot_pattern_context(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug, "limit": 5});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("gate_pattern.list_hot", Some(params));
    let Ok(serde_json::Value::Array(patterns)) = result else {
        return None;
    };
    if patterns.is_empty() {
        return None;
    }
    let mut ctx = String::from("\n[SELF_EVOLVE_PATTERNS]\nstatus: autonomous\n");
    for p in &patterns {
        let tokens = p.get("error_tokens").and_then(|v| v.as_str()).unwrap_or("");
        let fix = p.get("fix_strategy").and_then(|v| v.as_str()).unwrap_or("");
        let n = p
            .get("occurrence_count")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        writeln!(ctx, "pattern: {tokens} | fix: {fix} | occurrences: {n}").ok();
    }
    Some(ctx)
}
