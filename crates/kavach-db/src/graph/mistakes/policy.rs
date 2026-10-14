use crate::error::{Error, Result};
use crate::db::DbStore;

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
    db: &DbStore,
    name: &str,
    props: &DeployedPolicyProps,
) -> Result<String> {
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
    let props_json = serde_json::to_string(&properties)?;
    let id = format!("deployed_policy:{name}");
    let mut resp = db
        .exec_params(
            "INSERT INTO entities (id, entity_type, name, properties) \
             VALUES ($id, 'deployed_policy', $name, $props) \
             ON CONFLICT(id) DO UPDATE SET \
                 name = excluded.name, properties = excluded.properties \
             RETURNING id",
            libsql::params![id.clone(), name.to_owned(), props_json],
        )
        .await?;
    match resp.next().await? {
        Some(_) => Ok(id),
        None => Err(Error::RecordNotFound(format!(
            "deployed_policy upsert empty: {name}"
        ))),
    }
}

#[cfg(test)]
#[path = "policy_test.rs"]
mod policy_test;
