//! C2 proof: citation DAG edges (`cite`/`parent`/`depends_on`) relate, foreign
//! edges are rejected, and the single-query `<-cite` traversal returns citers.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{is_citation_rel, relate_citation, traverse_with_citations};
use crate::{apply_schema, open_memory};
use surrealdb_types::RecordId;

async fn seed() -> surrealdb::Surreal<surrealdb::engine::any::Any> {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db).await.expect("schema");
    db.query("CREATE project:p SET slug = 'p', name = 'p'")
        .await
        .expect("project");
    db.query(
        "CREATE citation:cit SET project = project:p, entry_key = 'surreal', name = 'SurrealDB', \
         metadata = [{ slug: 'records', url: 'https://surrealdb.com/docs' }]",
    )
    .await
    .expect("citation");
    db.query("CREATE decision:d SET project = project:p, entry_key = 'use-surreal', title = 't', content = 'c'")
        .await
        .expect("decision");
    db
}

#[test]
fn citation_rel_allowlist_is_exact() {
    assert!(is_citation_rel("cite"));
    assert!(is_citation_rel("parent"));
    assert!(is_citation_rel("depends_on"));
    assert!(!is_citation_rel("contains"), "workflow edge is not a citation edge");
    assert!(!is_citation_rel("is_a"), "ontology edge is not a citation edge");
}

#[tokio::test]
async fn decision_cites_citation_and_single_query_traversal_finds_it() {
    let db = seed().await;
    let from = RecordId::new("decision", "d");
    let to = RecordId::new("citation", "cit");
    relate_citation(&db, &from, &to, "cite", 1.0)
        .await
        .expect("cite edge accepted");

    let citing = traverse_with_citations(&db, &to)
        .await
        .expect("single-query traversal");
    assert_eq!(citing.len(), 1, "exactly the one citing decision is returned");
    assert_eq!(citing[0], from, "the citing node is the decision, in one round-trip");
}

#[tokio::test]
async fn foreign_edge_is_rejected_on_citation_relate() {
    let db = seed().await;
    let from = RecordId::new("decision", "d");
    let to = RecordId::new("citation", "cit");
    let res = relate_citation(&db, &from, &to, "contains", 1.0).await;
    assert!(res.is_err(), "a non-citation edge must be rejected by relate_citation");
}

#[tokio::test]
async fn citation_parent_edge_links_citations() {
    let db = seed().await;
    db.query(
        "CREATE citation:child SET project = project:p, entry_key = 'child', name = 'Child', \
         metadata = [{ slug: 'c', url: 'https://surrealdb.com/docs/child' }]",
    )
    .await
    .expect("child citation");
    let child = RecordId::new("citation", "child");
    let parent = RecordId::new("citation", "cit");
    relate_citation(&db, &child, &parent, "parent", 1.0)
        .await
        .expect("parent edge accepted");
}
