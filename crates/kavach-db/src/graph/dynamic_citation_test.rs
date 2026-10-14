//! C2 proof: citation DAG edges (`cite`/`parent`/`depends_on`) relate, foreign
//! edges are rejected, and the single-query `<-cite` traversal returns citers.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{is_citation_rel, relate_citation, traverse_with_citations};
use crate::db::DbStore;
use crate::{apply_schema, open_memory};

async fn seed() -> DbStore {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db).await.expect("schema");
    db.exec_params(
        "INSERT INTO citations (project, entry_key, name, slug, url, created_at, updated_at) \
         VALUES ('p', 'surreal', 'DbStoreDB', 'records', 'https://surrealdb.com/docs', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("citation");
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, created_at, updated_at) \
         VALUES ('p', 'decision', 'use-surreal', 't', 'c', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("decision");
    db
}

#[test]
fn citation_rel_allowlist_is_exact() {
    assert!(is_citation_rel("cite"));
    assert!(is_citation_rel("parent"));
    assert!(is_citation_rel("depends_on"));
    assert!(
        !is_citation_rel("contains"),
        "workflow edge is not a citation edge"
    );
    assert!(
        !is_citation_rel("is_a"),
        "ontology edge is not a citation edge"
    );
}

#[tokio::test]
async fn decision_cites_citation_and_single_query_traversal_finds_it() {
    let db = seed().await;
    let from = "memory:p/decision/use-surreal".to_owned();
    let to = "citation:p/surreal".to_owned();
    relate_citation(&db, &from, &to, "cite", 1.0)
        .await
        .expect("cite edge accepted");

    let citing = traverse_with_citations(&db, &to)
        .await
        .expect("single-query traversal");
    assert_eq!(
        citing.len(),
        1,
        "exactly the one citing decision is returned"
    );
    assert_eq!(
        citing[0], from,
        "the citing node is the decision, in one round-trip"
    );
}

#[tokio::test]
async fn foreign_edge_is_rejected_on_citation_relate() {
    let db = seed().await;
    let from = "memory:p/decision/use-surreal".to_owned();
    let to = "citation:p/surreal".to_owned();
    let res = relate_citation(&db, &from, &to, "contains", 1.0).await;
    assert!(
        res.is_err(),
        "a non-citation edge must be rejected by relate_citation"
    );
}

#[tokio::test]
async fn citation_parent_edge_links_citations() {
    let db = seed().await;
    db.exec_params(
        "INSERT INTO citations (project, entry_key, name, slug, url, created_at, updated_at) \
         VALUES ('p', 'child', 'Child', 'c', 'https://surrealdb.com/docs/child', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("child citation");
    let child = "citation:p/child".to_owned();
    let parent = "citation:p/surreal".to_owned();
    relate_citation(&db, &child, &parent, "parent", 1.0)
        .await
        .expect("parent edge accepted");
}
