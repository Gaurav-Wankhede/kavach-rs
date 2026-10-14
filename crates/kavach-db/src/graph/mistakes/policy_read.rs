// Read side of the deployed_policy node — SessionStart + CLI + GUI all read the
// SAME node db.policy_improve writes, closing any read/write split-brain.
//
//   handful, not millions) by pessimistic value (lcb), descending. CHOICE:
//   materialize all rows, then slice::sort_by (Rust stdlib stable sort,
//   Timsort-derived, O(N log N)). REJECTED: SQL `ORDER BY json_extract(...)`
//   — ordering on a nested-property alias is non-portable across libsql
//   versions; a partial sort gives no measurable win at N≈scopes.
//   TIME: O(N log N), N = deployed_policy count. SPACE: O(N). YEAR: 2026.
use crate::error::Result;
use crate::db::DbStore;

/// One deployed advisory policy: the promoted action distribution plus the
/// coverage/sample evidence, surfaced for reinjection (highest `lcb` first).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DeployedPolicyRow {
    /// Canonical scope name, e.g. `policy.advisory.global`.
    pub name: String,
    /// Promoted probability of `Allow`.
    pub allow: f64,
    /// Promoted probability of `Ask`.
    pub ask: f64,
    /// Promoted probability of `Block`.
    pub block: f64,
    /// Candidate pessimistic value that beat the incumbent.
    pub lcb: f64,
    /// `DataCOPE` coverage ratio (ESS/n) backing the promotion.
    pub coverage_ratio: f64,
    /// Reward-filled samples the promotion rested on.
    pub usable_samples: i64,
}

/// Top-N deployed policies ranked by `lcb` (descending), then by name for a
/// stable tie-break.
///
/// # Errors
/// Propagates `Error::Database` when the query fails.
pub async fn top_deployed_policies(
    db: &DbStore,
    limit: usize,
) -> Result<Vec<DeployedPolicyRow>> {
    let mut resp = db
        .exec_params(
            "SELECT name, \
             json_extract(properties, '$.allow') AS allow, \
             json_extract(properties, '$.ask') AS ask, \
             json_extract(properties, '$.block') AS block, \
             json_extract(properties, '$.lcb') AS lcb, \
             json_extract(properties, '$.coverage_ratio') AS coverage_ratio, \
             json_extract(properties, '$.usable_samples') AS usable_samples \
             FROM entities WHERE entity_type = 'deployed_policy'",
            (),
        )
        .await?;
    let mut rows: Vec<DeployedPolicyRow> = Vec::new();
    while let Some(r) = resp.next().await? {
        rows.push(DeployedPolicyRow {
            name: r.get::<String>(0)?,
            allow: r.get::<f64>(1)?,
            ask: r.get::<f64>(2)?,
            block: r.get::<f64>(3)?,
            lcb: r.get::<f64>(4)?,
            coverage_ratio: r.get::<f64>(5)?,
            usable_samples: r.get::<i64>(6)?,
        });
    }
    rows.sort_by(|a, b| {
        b.lcb
            .partial_cmp(&a.lcb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(limit);
    Ok(rows)
}
