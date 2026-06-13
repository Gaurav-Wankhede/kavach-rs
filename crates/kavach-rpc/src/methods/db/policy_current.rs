// `db.policy_current` — the learned-policy read side. SessionStart reinjection
// (and the CLI/GUI) call this so the harness is INFORMED by the deployed_policy
// node db.policy_improve writes — the advisory half of RL-in-the-loop.
//
// ALGO: none local — thin RPC pass-through; lcb-ranking + top-k is delegated to
//   kavach_surreal::graph_top_deployed_policies (see its ALGO note). Here we only
//   clamp the page size and map rows to the wire DTO. TIME: O(N) map. YEAR: 2026.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::graph_top_deployed_policies;
use serde::{Deserialize, Serialize};

/// Default page size when the caller omits `limit`.
const DEFAULT_LIMIT: u32 = 5;
/// Hard cap so a hostile/buggy caller cannot request an unbounded scan.
const MAX_LIMIT: u32 = 100;

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PolicyCurrentParams {
    /// Max deployed policies to return (clamped to `MAX_LIMIT`; defaults to `DEFAULT_LIMIT`).
    #[serde(default)]
    pub limit: Option<u32>,
}

impl PolicyCurrentParams {
    /// Construct params for a `db.policy_current` call.
    #[must_use]
    pub const fn new(limit: Option<u32>) -> Self {
        Self { limit }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PolicyDto {
    /// Canonical scope name, e.g. `policy.advisory.global`.
    pub name: String,
    /// Promoted probability of `Allow`.
    pub allow: f64,
    /// Promoted probability of `Ask`.
    pub ask: f64,
    /// Promoted probability of `Block`.
    pub block: f64,
    /// Candidate pessimistic value (LCB) that won promotion.
    pub lcb: f64,
    /// `DataCOPE` coverage ratio backing the promotion.
    pub coverage_ratio: f64,
    /// Reward-filled samples the promotion rested on.
    pub usable_samples: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PolicyCurrentResult {
    /// Deployed advisory policies ordered by LCB (descending).
    pub policies: Vec<PolicyDto>,
}

/// List the current deployed advisory policies from the knowledge graph.
///
/// # Errors
/// Returns an RPC error if the graph query fails.
pub async fn policy_current(
    state: &AppState,
    p: PolicyCurrentParams,
) -> Result<PolicyCurrentResult, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let rows = graph_top_deployed_policies(&state.db, limit)
        .await
        .map_err(surreal_to_rpc)?;
    let policies = rows
        .into_iter()
        .map(|r| PolicyDto {
            name: r.name,
            allow: r.allow,
            ask: r.ask,
            block: r.block,
            lcb: r.lcb,
            coverage_ratio: r.coverage_ratio,
            usable_samples: r.usable_samples,
        })
        .collect();
    Ok(PolicyCurrentResult { policies })
}
