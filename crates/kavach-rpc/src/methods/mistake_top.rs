// `mistake.top` — recurrence-ranked anti_pattern listing. The read side of the
// autonomous mistake loop: SessionStart reinjection, the CLI, and the GUI all
// call this so they surface the graph anti_patterns the daemon clusters, instead
// of the legacy `pattern` memory_entries the writers stopped populating.
// Split out of methods/mistake.rs to keep each leaf ≤100 LOC.
//
//   to kavach_surreal::graph_top_anti_patterns (see that module's ALGO note:
//   bounded in-memory sort). Here we only clamp the page size and map rows to the
//   wire DTO. TIME: O(N) map over the already-ranked slice. YEAR: 2026.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::graph_top_anti_patterns;
use serde::{Deserialize, Serialize};

/// Default page size when the caller omits `limit`.
const DEFAULT_LIMIT: u32 = 10;
/// Hard cap so a hostile/buggy caller cannot request an unbounded scan.
const MAX_LIMIT: u32 = 100;

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TopParams {
    /// Max anti-patterns to return (clamped to `MAX_LIMIT`; defaults to `DEFAULT_LIMIT`).
    #[serde(default)]
    pub limit: Option<u32>,
}

impl TopParams {
    /// Construct params for a `mistake.top` call.
    #[must_use]
    pub const fn new(limit: Option<u32>) -> Self {
        Self { limit }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AntiPatternDto {
    /// Canonical node name, e.g. `anti.continuation_menu.395f9852`.
    pub name: String,
    /// Gate that fired the originating mistakes.
    pub gate: String,
    /// The do-instead rule to reinforce (anti-parrot framing).
    pub correct_action: String,
    /// Recurrence count = inbound `instance_of` edges.
    pub hit_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TopResult {
    /// Anti-patterns ordered by recurrence (descending).
    pub patterns: Vec<AntiPatternDto>,
}

/// List the top-N anti-patterns by recurrence from the knowledge graph.
///
/// # Errors
/// Returns an RPC error if the graph query fails.
pub async fn top(state: &AppState, p: TopParams) -> Result<TopResult, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let rows = graph_top_anti_patterns(&state.db, limit)
        .await
        .map_err(surreal_to_rpc)?;
    let patterns = rows
        .into_iter()
        .map(|r| AntiPatternDto {
            name: r.name,
            gate: r.gate,
            correct_action: r.correct_action,
            hit_count: r.hit_count,
        })
        .collect();
    Ok(TopResult { patterns })
}
