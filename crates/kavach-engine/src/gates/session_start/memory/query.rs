// SOURCE: kavach decision.context-rot-surrealdb-pipeline
//! kavach-db lookups feeding the memory-bank context: ancestry chain + titles.
use crate::gates::context_compress;

pub(super) fn resolve_ancestry(project: &str) -> Vec<String> {
    let params = serde_json::json!({"slug": project});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("projects.ancestry", Some(params));
    let arr = match result {
        Ok(serde_json::Value::Array(a)) if !a.is_empty() => a,
        _ => return vec![project.to_owned()],
    };
    let compressed = context_compress::compress_db_rows(&arr, 10);
    let slugs: Vec<String> = compressed
        .iter()
        .filter_map(|v| {
            v.get("slug")
                .and_then(|s| s.as_str())
                .map(ToOwned::to_owned)
        })
        .collect();
    if slugs.is_empty() {
        vec![project.to_owned()]
    } else {
        slugs
    }
}

pub(super) fn query_category_titles(project: &str, category: &str) -> Option<String> {
    let params = serde_json::json!({
        "project": project, "category": category, "limit": 10,
    });
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("roadmap.list_titles", Some(params));
    let Ok(serde_json::Value::Array(arr)) = result else {
        return None;
    };
    if arr.is_empty() {
        return None;
    }
    let compressed = context_compress::compress_db_rows(&arr, 10);
    let lines: Vec<String> = compressed
        .iter()
        .filter_map(|v| {
            let cat = v.get("category")?.as_str()?;
            let key = v.get("key")?.as_str()?;
            let title = v.get("title")?.as_str()?;
            Some(format!("[{cat}] {key} — {title}"))
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    Some(lines.join("\n"))
}

/// Resolve the full ancestry chain for a project slug.
/// Returns [`child_slug`, `parent_slug`, ...] — child first.
/// Falls back to [project] if DB is unavailable or project not found.
pub(super) fn resolve_ancestry(project: &str) -> Vec<String> {
    let params = serde_json::json!({"slug": project});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("projects.ancestry", Some(params));
    let arr = match result {
        Ok(serde_json::Value::Array(a)) if !a.is_empty() => a,
        _ => return vec![project.to_owned()],
    };
    let slugs: Vec<String> = arr
        .iter()
        .filter_map(|v| {
            v.get("slug")
                .and_then(|s| s.as_str())
                .map(ToOwned::to_owned)
        })
        .collect();
    if slugs.is_empty() {
        vec![project.to_owned()]
    } else {
        slugs
    }
}

/// Query kavach-db titles only via DB API — no content bodies fetched.
/// Uses `list_titles_by_project` with LIMIT 10 per category.
pub(super) fn query_category_titles(project: &str, category: &str) -> Option<String> {
    let params = serde_json::json!({
        "project": project, "category": category, "limit": 10,
    });
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("roadmap.list_titles", Some(params));
    let Ok(serde_json::Value::Array(arr)) = result else {
        return None;
    };
    if arr.is_empty() {
        return None;
    }
    let lines: Vec<String> = arr
        .iter()
        .filter_map(|v| {
            let cat = v.get("category")?.as_str()?;
            let key = v.get("key")?.as_str()?;
            let title = v.get("title")?.as_str()?;
            Some(format!("[{cat}] {key} — {title}"))
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    Some(lines.join("\n"))
}
