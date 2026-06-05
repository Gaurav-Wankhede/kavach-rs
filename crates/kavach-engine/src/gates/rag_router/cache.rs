//! Process-wide `label -> Vec<RagTree>` cache + the RPC fetch/parse behind it.
//! Shared across every gate in one hook process so the tree JSON is parsed once.
use std::sync::{Mutex, OnceLock};

use kavach_rag_core::RagTree;

/// Empty Vec means "we tried and there was no row / parse failed". Subsequent
/// calls short-circuit without re-hitting `SurrealDB`.
type TreeCache = Mutex<Vec<(String, Vec<RagTree>)>>;

fn cache() -> &'static TreeCache {
    static CACHE: OnceLock<TreeCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Load (and cache) all trees for `label`. Degrades to an empty Vec on failure.
pub(in crate::gates::rag_router) fn load_trees(label: &str) -> Vec<RagTree> {
    if let Ok(guard) = cache().lock()
        && let Some((_, cached)) = guard.iter().find(|(k, _)| k == label)
    {
        return cached.clone();
    }
    let fetched = fetch_and_parse_all(label);
    if let Ok(mut guard2) = cache().lock()
        && !guard2.iter().any(|(k, _)| k == label)
    {
        guard2.push((label.to_owned(), fetched.clone()));
    }
    fetched
}

fn fetch_and_parse_all(label: &str) -> Vec<RagTree> {
    let params = serde_json::json!({"source": label});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("rag.tree_get", Some(params));
    let Ok(val) = result else {
        return Vec::new();
    };
    if !val.is_object() {
        return Vec::new();
    }
    let Some(bytes_val) = val.get("tree_json") else {
        return Vec::new();
    };
    // SurrealDB returns bytes as a JSON array of u8 numbers.
    let Some(byte_arr) = bytes_val.as_array() else {
        return Vec::new();
    };
    let bytes: Vec<u8> = byte_arr
        .iter()
        .filter_map(|byte_val| byte_val.as_u64().and_then(|n| u8::try_from(n).ok()))
        .collect();
    let Ok(body) = std::str::from_utf8(&bytes) else {
        return Vec::new();
    };
    let mut out: Vec<RagTree> = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(tree) = RagTree::from_json(line) {
            out.push(tree);
        }
    }
    out
}
