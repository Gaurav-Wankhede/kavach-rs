// Read side of the deployed_policy node — SessionStart + CLI + GUI all read the
// SAME node db.policy_improve writes, closing any read/write split-brain.
//
// ALGO: rank a bounded deployed_policy set (one node per advisory scope — a
//   handful, not millions) by pessimistic value (lcb), descending. CHOICE:
//   materialize all rows, then slice::sort_by (Rust stdlib stable sort,
//   Timsort-derived, O(N log N)). REJECTED: SurrealDB `ORDER BY properties.lcb`
//   — ordering on a nested-property alias is non-portable across SurrealDB
//   versions; a partial sort gives no measurable win at N≈scopes.
//   TIME: O(N log N), N = deployed_policy count. SPACE: O(N). YEAR: 2026.
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

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
/// Propagates `Error::Surreal` when the query fails (but a brand-new graph with
/// no `entity` table yet is the empty case, not an error).
pub async fn top_deployed_policies(db: &Surreal<Db>, limit: usize) -> Result<Vec<DeployedPolicyRow>> {
    #[derive(SurrealValue)]
    struct Row {
        name: String,
        allow: f64,
        ask: f64,
        block: f64,
        lcb: f64,
        coverage_ratio: f64,
        usable_samples: i64,
    }

    let q = "SELECT name, \
             properties.allow AS allow, properties.ask AS ask, \
             properties.block AS block, properties.lcb AS lcb, \
             properties.coverage_ratio AS coverage_ratio, \
             properties.usable_samples AS usable_samples \
             FROM entity WHERE entity_type = 'deployed_policy'";
    let mut resp = db.query(q).await?;
    // A brand-new graph never created the `entity` table, so SELECT raises
    // "table does not exist" — the empty case (zero policies), not a failure.
    let mut rows: Vec<Row> = match resp.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    rows.sort_by(|a, b| {
        b.lcb
            .partial_cmp(&a.lcb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(limit);
    Ok(rows
        .into_iter()
        .map(|r| DeployedPolicyRow {
            name: r.name,
            allow: r.allow,
            ask: r.ask,
            block: r.block,
            lcb: r.lcb,
            coverage_ratio: r.coverage_ratio,
            usable_samples: r.usable_samples,
        })
        .collect())
}
