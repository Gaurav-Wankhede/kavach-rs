//! Build search query from context and call `brain.think` RPC. Caches label→hits.
use std::sync::{Mutex, OnceLock};

/// Cache: label → hits as (score, id). Empty Vec means "tried and got zero hits".
type HitCache = Mutex<Vec<(String, Vec<(u32, String)>)>>;

fn cache() -> &'static HitCache {
    static CACHE: OnceLock<HitCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Query corpus via `brain.think` and return top-k hits as (score, id).
/// Degrades to empty Vec on RPC failure. Cached per label to avoid redundant queries.
pub(in crate::gates::rag_router) fn search_via_brain(
    label: &str,
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> Vec<(u32, String)> {
    // Check cache first.
    if let Ok(guard) = cache().lock()
        && let Some((_, cached)) = guard.iter().find(|(k, _)| k == label)
    {
        return cached.iter().take(top_k).cloned().collect();
    }

    // Build query from context: label, file path, intent, and raw_text snippet.
    // brain.think expects free-text; we construct a natural language query.
    let query = format!("[{label}] {intent} {file_path} {raw_text}");
    let fetched = brain_think_search(&query, top_k);

    // Cache the full result; callers truncate to their top_k.
    if let Ok(mut guard) = cache().lock()
        && !guard.iter().any(|(k, _)| k == label)
    {
        guard.push((label.to_owned(), fetched.clone()));
    }

    fetched.iter().take(top_k).cloned().collect()
}

fn brain_think_search(query: &str, limit: usize) -> Vec<(u32, String)> {
    let params = serde_json::json!({ "query": query, "limit": limit.max(1) });
    let result: Result<Vec<kavach_surreal::BrainHit>, _> =
        kavach_rpc::client::call("brain.think", Some(params));
    let Ok(hits) = result else {
        return Vec::new();
    };

    // Synthesize score from descending rank: top hit = len, next = len-1, etc.
    // This preserves the "higher score = better match" invariant of the old MatchResult.
    let len = u32::try_from(hits.len()).unwrap_or(u32::MAX);
    hits.into_iter()
        .enumerate()
        .map(|(idx, hit)| {
            let idx_u32 = u32::try_from(idx).unwrap_or(u32::MAX);
            let synthetic_score = len.saturating_sub(idx_u32).max(1);
            (synthetic_score, hit.id)
        })
        .collect()
}
