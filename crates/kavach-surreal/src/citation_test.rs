//! C3 proof: citation CRUD round-trips — `upsert_citation` creates then updates
//! on the UNIQUE (`project`, `entry_key`) key, `get_citation` bumps
//! `access_count`, `list_citations_by_project` returns rows, and the re-exported
//! `traverse` reaches a citer.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{
    CitationMeta, FRESHNESS_WINDOW_SECS, Freshness, STALE_MARKER, UpsertCitation, freshness,
    get_citation, list_citations_by_project, mark_if_stale, upsert_citation,
};
use crate::{apply_schema, open_memory};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb_types::RecordId;

async fn seed() -> Surreal<Any> {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db).await.expect("schema");
    db.query("CREATE project:p SET slug = 'p', name = 'p'")
        .await
        .expect("project");
    db
}

fn meta(slug: &str, url: &str) -> CitationMeta {
    CitationMeta {
        slug: slug.to_owned(),
        desc: String::new(),
        url: url.to_owned(),
        parent: None,
        depends_on: None,
        best_practice: String::new(),
        worst_practice: String::new(),
        tradeoff: String::new(),
    }
}

#[tokio::test]
async fn upsert_creates_then_updates_on_same_key() {
    let db = seed().await;
    let project = RecordId::new("project", "p");
    let first = upsert_citation(
        &db,
        &UpsertCitation {
            project: project.clone(),
            entry_key: "surreal",
            name: "SurrealDB",
            metadata: vec![meta("records", "https://surrealdb.com/docs")],
        },
    )
    .await
    .expect("create");

    let second = upsert_citation(
        &db,
        &UpsertCitation {
            project: project.clone(),
            entry_key: "surreal",
            name: "SurrealDB v3",
            metadata: vec![meta("graph", "https://surrealdb.com/docs/graph")],
        },
    )
    .await
    .expect("update");

    assert_eq!(first, second, "same key upserts the same row, never a second");
    let rows = list_citations_by_project(&db, &project).await.expect("list");
    assert_eq!(rows.len(), 1, "the UNIQUE index kept exactly one row");
    assert_eq!(rows[0].name, "SurrealDB v3", "name refreshed on update");
    assert_eq!(rows[0].metadata[0].slug, "graph", "metadata replaced on update");
}

#[tokio::test]
async fn get_bumps_access_count() {
    let db = seed().await;
    let project = RecordId::new("project", "p");
    upsert_citation(
        &db,
        &UpsertCitation {
            project: project.clone(),
            entry_key: "axum",
            name: "Axum",
            metadata: vec![meta("handler", "https://docs.rs/axum")],
        },
    )
    .await
    .expect("create");

    let first = get_citation(&db, &project, "axum").await.expect("get").expect("present");
    assert_eq!(first.access_count, 1, "first read bumps to 1");
    let second = get_citation(&db, &project, "axum").await.expect("get").expect("present");
    assert_eq!(second.access_count, 2, "second read bumps to 2");
}

#[tokio::test]
async fn get_missing_is_none_not_err() {
    let db = seed().await;
    let project = RecordId::new("project", "p");
    let got = get_citation(&db, &project, "nope").await.expect("query ok");
    assert!(got.is_none(), "absent key yields None, not Err");
}

#[tokio::test]
async fn traverse_reexport_finds_a_citer() {
    let db = seed().await;
    let project = RecordId::new("project", "p");
    upsert_citation(
        &db,
        &UpsertCitation {
            project,
            entry_key: "surreal",
            name: "SurrealDB",
            metadata: vec![meta("records", "https://surrealdb.com/docs")],
        },
    )
    .await
    .expect("create");
    db.query("CREATE decision:d SET project = project:p, entry_key = 'use-surreal', title = 't', content = 'c'")
        .await
        .expect("decision");
    let from = RecordId::new("decision", "d");
    let to: RecordId = db
        .query("SELECT VALUE id FROM citation WHERE entry_key = 'surreal' LIMIT 1")
        .await
        .expect("cite id")
        .take::<Vec<RecordId>>(0)
        .expect("take")
        .pop()
        .expect("one citation");
    crate::graph::relate_citation(&db, &from, &to, "cite", 1.0)
        .await
        .expect("cite edge");
    let citers = super::traverse(&db, &to).await.expect("traverse");
    assert_eq!(citers, vec![from], "the citing decision is reached in one query");
}

#[test]
fn freshness_window_boundary_is_exact() {
    let now = 1_000_000_000;
    assert_eq!(freshness(Some(now), now), Freshness::Fresh, "just-updated is fresh");
    assert_eq!(
        freshness(Some(now - FRESHNESS_WINDOW_SECS + 1), now),
        Freshness::Fresh,
        "one second inside the window is fresh"
    );
    assert_eq!(
        freshness(Some(now - FRESHNESS_WINDOW_SECS), now),
        Freshness::Stale,
        "exactly at the window edge is stale"
    );
    assert_eq!(
        freshness(Some(now - FRESHNESS_WINDOW_SECS - 1), now),
        Freshness::Stale,
        "past the window is stale"
    );
}

#[test]
fn freshness_unstamped_and_future_are_stale() {
    let now = 1_000_000_000;
    assert_eq!(freshness(None, now), Freshness::Stale, "never-stamped is not trusted");
    assert_eq!(
        freshness(Some(now + 60), now),
        Freshness::Stale,
        "a future timestamp (clock skew) is not trusted"
    );
}

#[test]
fn mark_if_stale_flags_only_stale() {
    assert_eq!(mark_if_stale(Freshness::Fresh, "docs"), "docs", "fresh text is untouched");
    let marked = mark_if_stale(Freshness::Stale, "docs");
    assert!(marked.starts_with(STALE_MARKER), "stale text is prefixed with the marker");
    assert!(marked.contains("docs"), "stale text still carries the content");
}
