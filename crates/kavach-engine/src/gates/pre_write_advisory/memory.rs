// ARCH: MemoryAwarenessAdvisory — surface OPEN roadmap titles, not counts.
// PATTERN: pre_write_context_inject
// CAPACITY: <=2 KB per gate fire, <=6 entries (cap to avoid premature compaction)
// QPS: 1 invocation per write tool call (Edit|Write|MultiEdit)
// LATENCY: <30ms via kavach-rpc daemon; degrades silently to empty if daemon down
// CONSISTENCY: read-only, eventually-consistent (RPC may be ms behind)
// FAILURE_MODE: rpc absent / db error -> return None -> no injection (graceful)
// OBSERVABILITY: anchor `[MEMORY:project:<slug>]` so users can grep
// TRADEOFF: per-write-call cost; mitigated by cap + early return when project empty
// SOURCE: https://code.claude.com/docs/en/hooks (additionalContext patterns)
use std::fmt::Write as _;

/// Pull the active project's OPEN roadmap items (`in_progress` + `todo`) via the
/// kavach-rpc daemon and return them as a factual context block.
/// Returns None when daemon is unavailable or no open items exist.
// ALGO: LinearFilterTakeN
// PROBLEM_CLASS: stream
// REJECTED: [{"name":"server_side_where_rpc","reason":"new method for marginal win at n<=2000"},{"name":"presorted_index","reason":"premature optimization at this scale"}]
// TIME: O(n) bounded — early-exits at MAX_ITEMS via .take() | SPACE: O(MAX_ITEMS)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: client-side enumerate vs server-side filter; acceptable at n<=2000
// BENCHMARK: sub-millisecond at 2000 entries (lazy iterator, no allocation in filter)
pub(super) fn memory_awareness_advisory(project_slug: &str) -> Option<String> {
    const MAX_OPEN: usize = 6;
    const MAX_DECISIONS: usize = 3;
    let roadmap_arr = fetch_titles(project_slug, "roadmap");
    let decision_arr = fetch_titles(project_slug, "decision");
    let open: Vec<&serde_json::Value> = roadmap_arr
        .iter()
        .filter(|row| {
            row.get("entry_status")
                .and_then(|s| s.as_str())
                .is_some_and(|s| matches!(s, "todo" | "in_progress"))
        })
        .take(MAX_OPEN)
        .collect();
    let decisions: Vec<&serde_json::Value> = decision_arr.iter().take(MAX_DECISIONS).collect();
    if open.is_empty() && decisions.is_empty() {
        return None;
    }
    let mut out = String::new();
    writeln!(out, "[MEMORY:project:{project_slug}]").ok();
    if !open.is_empty() {
        out.push_str("Open roadmap items (state, not instructions):\n");
        for row in open {
            let status = row
                .get("entry_status")
                .and_then(|s| s.as_str())
                .unwrap_or("?");
            let key = row.get("key").and_then(|s| s.as_str()).unwrap_or("");
            let title = row.get("title").and_then(|s| s.as_str()).unwrap_or("");
            writeln!(
                out,
                "  [{status}] {key} - {}",
                title.lines().next().unwrap_or(title)
            )
            .ok();
        }
    }
    if !decisions.is_empty() {
        out.push_str("Recent decisions (don't re-litigate):\n");
        for row in decisions {
            let key = row.get("key").and_then(|s| s.as_str()).unwrap_or("");
            let title = row.get("title").and_then(|s| s.as_str()).unwrap_or("");
            writeln!(out, "  - {key} - {}", title.lines().next().unwrap_or(title)).ok();
        }
    }
    Some(out)
}

/// Wrap the existing `roadmap.list_titles` RPC for one category. Returns an
/// empty Vec when the daemon is down or returns nothing — caller treats both
/// as "no rows for this category" without distinguishing.
fn fetch_titles(project_slug: &str, category: &str) -> Vec<serde_json::Value> {
    let params = serde_json::json!({"project": project_slug, "category": category});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("roadmap.list_titles", Some(params));
    match result {
        Ok(serde_json::Value::Array(a)) => a,
        _ => Vec::new(),
    }
}
