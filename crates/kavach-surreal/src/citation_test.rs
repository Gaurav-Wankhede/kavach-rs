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
    Citation, CitationMeta, FRESHNESS_WINDOW_SECS, Freshness, STALE_MARKER, UpsertCitation,
    citations_cited_by, citations_for_nodes, freshness, get_citation, list_citations_by_project,
    mark_if_stale, merge_node_into_citation, plan_refresh, reward_citation_edges, upsert_citation,
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

fn cite(entry_key: &str, name: &str, url: &str, updated_unix: Option<i64>) -> Citation {
    Citation {
        id: None,
        project: RecordId::new("project", "p"),
        entry_key: entry_key.to_owned(),
        name: name.to_owned(),
        metadata: vec![meta("s", url)],
        access_count: 0,
        created_unix: updated_unix,
        updated_unix,
    }
}

#[test]
fn plan_refresh_partitions_stale_and_serves_all() {
    let now = 2_000_000_000;
    let fresh = cite("axum", "Axum", "https://docs.rs/axum", Some(now));
    let stale = cite("surreal", "SurrealDB", "https://surrealdb.com/docs", Some(now - FRESHNESS_WINDOW_SECS - 1));
    let plan = plan_refresh(&[fresh, stale], now);

    assert_eq!(plan.refresh.len(), 1, "only the stale citation is queued for re-research");
    assert_eq!(plan.refresh[0].entry_key, "surreal", "the stale one is named");
    assert_eq!(plan.refresh[0].urls, vec!["https://surrealdb.com/docs".to_owned()], "its docs URL is the fetch target");

    assert_eq!(plan.served.len(), 2, "every recalled citation is still served this turn");
    assert_eq!(plan.served[0], "Axum", "fresh content serves clean");
    assert!(plan.served[1].starts_with(STALE_MARKER), "stale content serves marked, never blocks on the network");
}

#[test]
fn plan_refresh_empty_recall_is_empty_plan() {
    let plan = plan_refresh(&[], 2_000_000_000);
    assert!(plan.refresh.is_empty() && plan.served.is_empty(), "no recall -> no work, no served text");
}

#[tokio::test]
async fn merge_node_wires_cite_edge_non_destructively() {
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
    .expect("citation");
    db.query("CREATE entity:ap SET entity_type = 'anti_pattern', name = 'await-swallows-err'")
        .await
        .expect("anti_pattern node");

    let node = RecordId::new("entity", "ap");
    let citation: RecordId = db
        .query("SELECT VALUE id FROM citation WHERE entry_key = 'surreal' LIMIT 1")
        .await
        .expect("cite id")
        .take::<Vec<RecordId>>(0)
        .expect("take")
        .pop()
        .expect("one citation");

    merge_node_into_citation(&db, &node, &citation).await.expect("merge edge");

    let cited = citations_cited_by(&db, &node).await.expect("forward walk");
    assert_eq!(cited, vec![citation.clone()], "the node reaches its merged citation in one query");

    let still_there: Vec<RecordId> = db
        .query("SELECT VALUE id FROM entity WHERE entity_type = 'anti_pattern'")
        .await
        .expect("entity still queryable")
        .take::<Vec<RecordId>>(0)
        .expect("take");
    assert_eq!(still_there, vec![node.clone()], "the anti_pattern row is untouched — merge is an edge, not a move");

    merge_node_into_citation(&db, &node, &citation).await.expect("replayed merge");
    let edges: Vec<RecordId> = db
        .query("SELECT VALUE id FROM cite WHERE in = $node AND out = $cit")
        .bind(("node", node))
        .bind(("cit", citation))
        .await
        .expect("edge count")
        .take::<Vec<RecordId>>(0)
        .expect("take");
    assert_eq!(edges.len(), 1, "replayed merge is idempotent — exactly one cite edge, no double-count");
}

#[tokio::test]
async fn citations_for_nodes_dedupes_across_recalled_rows() {
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
    .expect("citation");
    let citation: RecordId = db
        .query("SELECT VALUE id FROM citation WHERE entry_key = 'surreal' LIMIT 1")
        .await
        .expect("cite id")
        .take::<Vec<RecordId>>(0)
        .expect("take")
        .pop()
        .expect("one citation");
    db.query("CREATE decision:d SET project = project:p, entry_key = 'k1', title = 't', content = 'c'")
        .await
        .expect("decision");
    db.query("CREATE roadmap:r SET project = project:p, entry_key = 'k2', title = 't', content = 'c'")
        .await
        .expect("roadmap");
    let decision = RecordId::new("decision", "d");
    let roadmap = RecordId::new("roadmap", "r");
    merge_node_into_citation(&db, &decision, &citation).await.expect("merge decision");
    merge_node_into_citation(&db, &roadmap, &citation).await.expect("merge roadmap");

    let cits = citations_for_nodes(&db, &[decision, roadmap]).await.expect("batch recall");
    assert_eq!(cits, vec![citation], "both recalled rows cite the same citation -> deduped to one");

    let empty = citations_for_nodes(&db, &[]).await.expect("empty batch");
    assert!(empty.is_empty(), "no recalled nodes -> no citations");
}

#[tokio::test]
async fn reward_flows_along_cite_edges() {
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
    .expect("citation");
    let citation: RecordId = db
        .query("SELECT VALUE id FROM citation WHERE entry_key = 'surreal' LIMIT 1")
        .await
        .expect("cite id")
        .take::<Vec<RecordId>>(0)
        .expect("take")
        .pop()
        .expect("one citation");
    db.query("CREATE decision:d SET project = project:p, entry_key = 'k', title = 't', content = 'c'")
        .await
        .expect("decision");
    let node = RecordId::new("decision", "d");
    merge_node_into_citation(&db, &node, &citation).await.expect("merge");

    let n = reward_citation_edges(&db, &citation, 0.5).await.expect("reward");
    assert_eq!(n, 1, "the single cite edge into the citation is rewarded");

    let weights: Vec<f64> = db
        .query("SELECT VALUE weight FROM cite WHERE out = $cit")
        .bind(("cit", citation.clone()))
        .await
        .expect("weights")
        .take::<Vec<f64>>(0)
        .expect("take");
    assert_eq!(weights, vec![1.5], "edge weight climbed from the merge default 1.0 by delta 0.5");

    let unlinked = reward_citation_edges(&db, &RecordId::new("citation", "absent"), 1.0)
        .await
        .expect("reward on unlinked citation");
    assert_eq!(unlinked, 0, "a citation with no cite edges rewards nothing, no error");
}
