// Shared JS-literal escaping for every pattern emitter under `compile/`.
// SOURCE: decision.goal-oracle-workflow · serde_json emits a quoted, fully
// escaped JS string literal, defeating template-injection via goal text.

/// JSON-escape a string for safe interpolation into generated JS source.
pub(super) fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_owned())
}
