//! Fetch the project's DYNAMIC dispatch directive from the kavach DB.
//!
//! The stop-gate emits this operator-editable text (a `decision` row) instead of
//! procedure prose compiled into the binary — change gate behavior per-project
//! without a rebuild.

/// Directive text for `project_slug`'s `key` row, or `None` (absent row, empty
/// content, or daemon unreachable → caller uses a minimal fallback). A single
/// best-effort `db.get`; no self-heal/spawn (the dispatch path already warmed the
/// daemon via `rpc_next`).
pub(in crate::gates::stop_dispatch) fn rpc_get_directive(
    project_slug: &str,
    key: &str,
) -> Option<String> {
    let mut map = serde_json::Map::new();
    map.insert("project".to_owned(), serde_json::Value::String(project_slug.to_owned()));
    map.insert("category".to_owned(), serde_json::Value::String("decision".to_owned()));
    map.insert("key".to_owned(), serde_json::Value::String(key.to_owned()));
    map.insert("full".to_owned(), serde_json::Value::Bool(true));
    let params = serde_json::Value::Object(map);
    let v = kavach_rpc::client::call::<_, serde_json::Value>("db.get", Some(params)).ok()?;
    if v.get("found").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }
    v.get("entry")
        .and_then(|e| e.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
}
