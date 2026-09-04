// Proves the deployed_policy upsert/read pair: a scope is a singleton (re-upsert
// overwrites, not duplicates — backed by the UNIQUE(entity_type,name) index),
// the read ranks by lcb descending, a fresh graph reads empty, and an empty name
// fails closed. The read side of the loop db.policy_improve writes.
use super::super::policy_read::top_deployed_policies;
use super::{DeployedPolicyProps, upsert_deployed_policy};
use crate::open_memory;

/// Build props with the given action-Allow weight and pessimistic value.
fn props(allow: f64, lcb: f64) -> DeployedPolicyProps {
    DeployedPolicyProps {
        allow,
        ask: 0.2,
        block: 0.1,
        lcb,
        incumbent_lcb: 0.0,
        coverage_ratio: 0.5,
        usable_samples: 42,
    }
}

#[tokio::test]
async fn upsert_is_singleton_then_read_returns_latest() {
    let db = open_memory().await.expect("open in-memory db");
    upsert_deployed_policy(&db, "policy.advisory.global", &props(0.7, 0.5))
        .await
        .expect("first upsert");
    // Re-upsert the SAME scope with a different distribution.
    upsert_deployed_policy(&db, "policy.advisory.global", &props(0.2, 0.6))
        .await
        .expect("re-upsert same scope");

    let top = top_deployed_policies(&db, 10).await.expect("read policies");
    assert_eq!(top.len(), 1, "same scope must be a singleton, got {top:?}");
    assert!(
        (top[0].allow - 0.2).abs() < f64::EPSILON,
        "read must return the LATEST upsert"
    );
}

#[tokio::test]
async fn ranks_by_lcb_descending() {
    let db = open_memory().await.expect("open in-memory db");
    upsert_deployed_policy(&db, "policy.advisory.global", &props(0.5, 0.5))
        .await
        .expect("upsert global");
    upsert_deployed_policy(&db, "policy.advisory.bash", &props(0.5, 0.9))
        .await
        .expect("upsert bash");

    let top = top_deployed_policies(&db, 10).await.expect("read policies");
    assert_eq!(top.len(), 2, "two distinct scopes expected, got {top:?}");
    assert_eq!(
        top[0].name, "policy.advisory.bash",
        "higher lcb must rank first"
    );
    assert!((top[0].lcb - 0.9).abs() < f64::EPSILON);
}

#[tokio::test]
async fn empty_graph_returns_no_rows() {
    let db = open_memory().await.expect("open in-memory db");
    let top = top_deployed_policies(&db, 10).await.expect("read policies");
    assert!(top.is_empty(), "no deployed_policy ⇒ empty result");
}

#[tokio::test]
async fn empty_name_is_rejected() {
    let db = open_memory().await.expect("open in-memory db");
    let err = upsert_deployed_policy(&db, "", &props(0.5, 0.5)).await;
    assert!(err.is_err(), "empty name must fail closed");
}
