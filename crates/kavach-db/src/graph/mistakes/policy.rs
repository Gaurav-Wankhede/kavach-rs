// Learned-policy persistence — the `deployed_policy` node of the RLVR loop.
// db.policy_improve UPSERTs one versioned singleton per advisory scope here
// (name = 'policy.advisory.<scope>') ONLY after the three-gate AND: trust
// coverage >= floor, ope_audit promotable, controller::promote LCB win.
// SessionStart reads it back (policy_read::top_deployed_policies) and reinjects
// it as advisory context. Mirrors the anti_pattern upsert; same single-writer
// invariant (daemon-held &state.db only, never open_default()).
// See roadmap/unit.harness-rl.p6-policy-promotion-loop.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

/// The promoted advisory policy distribution plus the OPE evidence that won it.
///
/// `allow`/`ask`/`block` are the per-action probabilities of the promoted
/// candidate; `lcb`/`incumbent_lcb`/`coverage_ratio`/`usable_samples` are the
/// audit-gate witnesses recorded so the reinjected summary can show WHY it won.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DeployedPolicyProps {
    /// Promoted candidate probability of `Allow`.
    pub allow: f64,
    /// Promoted candidate probability of `Ask`.
    pub ask: f64,
    /// Promoted candidate probability of `Block`.
    pub block: f64,
    /// Candidate pessimistic value (lower confidence bound) that beat incumbent.
    pub lcb: f64,
    /// Incumbent LCB at promotion time (the bar the candidate cleared).
    pub incumbent_lcb: f64,
    /// `DataCOPE` coverage ratio (ESS/n) of the candidate over the logged data.
    pub coverage_ratio: f64,
    /// Reward-filled samples the promotion rested on.
    pub usable_samples: usize,
}

impl DeployedPolicyProps {
    /// Construct the promoted-policy properties (the cross-crate entry point —
    /// `db.policy_improve` builds this, the struct being `#[non_exhaustive]`).
    #[must_use]
    pub const fn new(
        allow: f64,
        ask: f64,
        block: f64,
        lcb: f64,
        incumbent_lcb: f64,
        coverage_ratio: f64,
        usable_samples: usize,
    ) -> Self {
        Self {
            allow,
            ask,
            block,
            lcb,
            incumbent_lcb,
            coverage_ratio,
            usable_samples,
        }
    }
}

/// Upsert the deployed policy for one scope (a versioned singleton keyed by
/// `name`); overwrites the prior policy for that scope on each promotion.
///
/// # Errors
/// `Error::Migration` if `name` is empty; `Error::RecordNotFound` if the upsert
/// returns no row.
pub async fn upsert_deployed_policy(
    db: &Surreal<Db>,
    name: &str,
    props: &DeployedPolicyProps,
) -> Result<RecordId> {
    #[derive(SurrealValue)]
    struct IdRow {
        id: RecordId,
    }

    if name.is_empty() {
        return Err(Error::Migration(
            "deployed_policy name cannot be empty".into(),
        ));
    }
    let properties = serde_json::json!({
        "allow": props.allow,
        "ask": props.ask,
        "block": props.block,
        "lcb": props.lcb,
        "incumbent_lcb": props.incumbent_lcb,
        "coverage_ratio": props.coverage_ratio,
        "usable_samples": props.usable_samples,
    });
    let q = "UPSERT entity \
             SET entity_type = 'deployed_policy', name = $name, properties = $props, \
                 updated_at = time::now() \
             WHERE entity_type = 'deployed_policy' AND name = $name \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("name", name.to_owned()))
        .bind(("props", properties))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("deployed_policy upsert empty: {name}")))
}

#[cfg(test)]
#[path = "policy_test.rs"]
mod policy_test;
