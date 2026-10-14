//! C3 proof: citation CRUD round-trips — `upsert_citation` creates then updates
//! on the UNIQUE (`project`, `entry_key`) key, `get_citation` bumps
//! `access_count`, `list_citations_by_project` returns rows, and the re-exported
//! `traverse` reaches a citer.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{
    Citation, CitationMeta, FRESHNESS_WINDOW_SECS, Freshness, STALE_MARKER, UpsertCitation,
    citations_cited_by, citations_for_nodes, freshness, get_citation, list_citations_by_project,
    mark_if_stale, merge_metadata, merge_node_into_citation, plan_refresh,
    reward_citation_edges, upsert_citation,
};
use crate::db::DbStore;
use crate::db::entries::EntryStore;
use crate::{apply_schema, open_memory};

/// `merge_metadata` is the pure core of the accumulate-not-replace fix; pin its
/// contract directly so a regression names the rule, not just the round-trip.
#[test]
fn merge_metadata_accumulates_by_slug_and_refreshes_in_place() {
    let a = meta("a", "https://example.com/a1");
    let b = meta("b", "https://example.com/b");
    // Fresh slug appends.
    let m = merge_metadata(std::slice::from_ref(&a), std::slice::from_ref(&b));
    assert_eq!(m.len(), 2, "new slug accumulates");
    assert_eq!(m[0].slug, "a", "prior entry stays at its position");
    // Existing slug refreshes in place, does not duplicate.
    let a2 = meta("a", "https://example.com/a2");
    let m = merge_metadata(&m, std::slice::from_ref(&a2));
    assert_eq!(m.len(), 2, "same slug does not duplicate");
    assert_eq!(m[0].url, "https://example.com/a2", "refresh wins");
    assert_eq!(m[1].slug, "b", "the other entry is untouched");
    // Empty base accepts the first entry.
    assert_eq!(
        merge_metadata(&[], std::slice::from_ref(&a)).len(),
        1,
        "empty base seeds from the incoming entry"
    );
}

async fn seed() -> DbStore {
    let db = open_memory().await.expect("mem db");
    apply_schema(&db).await.expect("schema");
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
    let project = "p".to_owned();
    let first = upsert_citation(
        &db,
        &UpsertCitation {
            project: project.clone(),
            entry_key: "surreal",
            name: "DbStoreDB",
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
            name: "DbStoreDB v3",
            metadata: vec![meta("graph", "https://surrealdb.com/docs/graph")],
        },
    )
    .await
    .expect("update");

    assert_eq!(
        first, second,
        "same key upserts the same row, never a second"
    );
    let rows = list_citations_by_project(&db, &project)
        .await
        .expect("list");
    assert_eq!(rows.len(), 1, "the UNIQUE index kept exactly one row");
    assert_eq!(rows[0].name, "DbStoreDB v3", "name refreshed on update");
    // MERGE by slug, not replace: pinning a second source to the same entry_key
    // must ACCUMULATE. Replace-semantics silently discarded every previously
    // pinned source (`r-kavach-citation-add-upsert-by-entry-key`: six `citation-add`
    // calls, six `ok` messages, one surviving URL).
    let slugs: Vec<&str> = rows[0].metadata.iter().map(|m| m.slug.as_str()).collect();
    assert_eq!(
        slugs,
        vec!["records", "graph"],
        "distinct slugs accumulate in pin order; none discarded"
    );
    assert_eq!(
        rows[0].metadata[0].url, "https://surrealdb.com/docs",
        "the earlier source is untouched, not clobbered"
    );
}

/// REGRESSION (`r-kavach-citation-add-upsert-by-entry-key`): pinning N sources to
/// ONE `entry_key` must store all N. The bug reported `ok` for each insert while
/// keeping one, and left `name`/`slug`/`url` from three different inserts —
/// a silently WRONG citation.
#[tokio::test]
async fn six_sources_on_one_entry_key_all_survive() {
    let db = seed().await;
    let project = "p".to_owned();
    let sources = [
        ("fusion-3d-nodes", "https://example.com/3d-nodes"),
        ("fusion-spline-editor", "https://example.com/spline"),
        ("fusion-transform-3d", "https://example.com/transform"),
        ("fusion-sdk-euler", "https://example.com/euler"),
        ("blackmagic-forum", "https://example.com/forum"),
        ("resolve-scripting", "https://example.com/scripting"),
    ];
    for (slug, url) in sources {
        upsert_citation(
            &db,
            &UpsertCitation {
                project: project.clone(),
                entry_key: "research.canon",
                name: "Fusion Canon",
                metadata: vec![meta(slug, url)],
            },
        )
        .await
        .expect("each add reports success");
    }
    let rows = list_citations_by_project(&db, &project)
        .await
        .expect("list");
    assert_eq!(rows.len(), 1, "still ONE row (one entry_key)");
    assert_eq!(
        rows[0].metadata.len(),
        sources.len(),
        "all six sources survive — the whole point"
    );
    let stored: Vec<(&str, &str)> = rows[0]
        .metadata
        .iter()
        .map(|m| (m.slug.as_str(), m.url.as_str()))
        .collect();
    assert_eq!(
        stored,
        sources.to_vec(),
        "every (slug, url) round-trips exactly"
    );
}

/// Re-pinning an EXISTING slug refreshes that entry in place rather than
/// appending a duplicate.
#[tokio::test]
async fn repinning_same_slug_refreshes_in_place() {
    let db = seed().await;
    let project = "p".to_owned();
    for url in ["https://example.com/v1", "https://example.com/v2"] {
        upsert_citation(
            &db,
            &UpsertCitation {
                project: project.clone(),
                entry_key: "k",
                name: "n",
                metadata: vec![meta("only", url)],
            },
        )
        .await
        .expect("upsert");
    }
    let rows = list_citations_by_project(&db, &project).await.expect("list");
    assert_eq!(rows[0].metadata.len(), 1, "same slug does not duplicate");
    assert_eq!(
        rows[0].metadata[0].url, "https://example.com/v2",
        "the refresh wins"
    );
}

/// The denormalized `slug`/`url` columns are a legacy projection of `metadata[0]`
/// read directly by the direct-store `list_citations`. They must track it on EVERY
/// path, or the direct listing reports a source that is not what `name` claims.
#[tokio::test]
async fn denormalized_slug_url_track_metadata_zero() {
    let db = seed().await;
    for (slug, url) in [("first", "https://example.com/1"), ("second", "https://example.com/2")] {
        db.upsert_citation("p", "k", "N", slug, url)
            .await
            .expect("direct-store upsert");
    }
    let rows = db.list_citations("p").await.expect("direct list");
    assert_eq!(rows.len(), 1, "one row");
    assert_eq!(rows[0].slug, "first", "denormalized slug tracks metadata[0]");
    assert_eq!(
        rows[0].url, "https://example.com/1",
        "denormalized url tracks metadata[0] — not the LAST pin"
    );
}

#[tokio::test]
async fn get_bumps_access_count() {
    let db = seed().await;
    let project = "p".to_owned();
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

    let first = get_citation(&db, &project, "axum")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(first.access_count, 1, "first read bumps to 1");
    let second = get_citation(&db, &project, "axum")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(second.access_count, 2, "second read bumps to 2");
}

#[tokio::test]
async fn get_missing_is_none_not_err() {
    let db = seed().await;
    let project = "p".to_owned();
    let got = get_citation(&db, &project, "nope").await.expect("query ok");
    assert!(got.is_none(), "absent key yields None, not Err");
}

#[tokio::test]
async fn traverse_reexport_finds_a_citer() {
    let db = seed().await;
    let project = "p".to_owned();
    upsert_citation(
        &db,
        &UpsertCitation {
            project,
            entry_key: "surreal",
            name: "DbStoreDB",
            metadata: vec![meta("records", "https://surrealdb.com/docs")],
        },
    )
    .await
    .expect("create");
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, created_at, updated_at) \
         VALUES ('p', 'decision', 'use-surreal', 't', 'c', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("decision");
    let from = "memory:p/decision/use-surreal".to_owned();
    let to = "citation:p/surreal".to_owned();
    crate::graph::relate_citation(&db, &from, &to, "cite", 1.0)
        .await
        .expect("cite edge");
    let citers = super::traverse(&db, &to).await.expect("traverse");
    assert_eq!(
        citers,
        vec![from],
        "the citing decision is reached in one query"
    );
}

#[test]
fn freshness_window_boundary_is_exact() {
    let now = 1_000_000_000;
    assert_eq!(
        freshness(Some(now), now),
        Freshness::Fresh,
        "just-updated is fresh"
    );
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
    assert_eq!(
        freshness(None, now),
        Freshness::Stale,
        "never-stamped is not trusted"
    );
    assert_eq!(
        freshness(Some(now + 60), now),
        Freshness::Stale,
        "a future timestamp (clock skew) is not trusted"
    );
}

#[test]
fn mark_if_stale_flags_only_stale() {
    assert_eq!(
        mark_if_stale(Freshness::Fresh, "docs"),
        "docs",
        "fresh text is untouched"
    );
    let marked = mark_if_stale(Freshness::Stale, "docs");
    assert!(
        marked.starts_with(STALE_MARKER),
        "stale text is prefixed with the marker"
    );
    assert!(
        marked.contains("docs"),
        "stale text still carries the content"
    );
}

fn cite(entry_key: &str, name: &str, url: &str, updated_unix: Option<i64>) -> Citation {
    Citation {
        id: None,
        project: "p".to_owned(),
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
    let stale = cite(
        "surreal",
        "DbStoreDB",
        "https://surrealdb.com/docs",
        Some(now - FRESHNESS_WINDOW_SECS - 1),
    );
    let plan = plan_refresh(&[fresh, stale], now);

    assert_eq!(
        plan.refresh.len(),
        1,
        "only the stale citation is queued for re-research"
    );
    assert_eq!(
        plan.refresh[0].entry_key, "surreal",
        "the stale one is named"
    );
    assert_eq!(
        plan.refresh[0].urls,
        vec!["https://surrealdb.com/docs".to_owned()],
        "its docs URL is the fetch target"
    );

    assert_eq!(
        plan.served.len(),
        2,
        "every recalled citation is still served this turn"
    );
    assert_eq!(plan.served[0], "Axum", "fresh content serves clean");
    assert!(
        plan.served[1].starts_with(STALE_MARKER),
        "stale content serves marked, never blocks on the network"
    );
}

#[test]
fn plan_refresh_empty_recall_is_empty_plan() {
    let plan = plan_refresh(&[], 2_000_000_000);
    assert!(
        plan.refresh.is_empty() && plan.served.is_empty(),
        "no recall -> no work, no served text"
    );
}

#[tokio::test]
async fn merge_node_wires_cite_edge_non_destructively() {
    let db = seed().await;
    let project = "p".to_owned();
    upsert_citation(
        &db,
        &UpsertCitation {
            project,
            entry_key: "surreal",
            name: "DbStoreDB",
            metadata: vec![meta("records", "https://surrealdb.com/docs")],
        },
    )
    .await
    .expect("citation");
    db.exec_params(
        "INSERT INTO entities (id, entity_type, name) VALUES ('entity:ap', 'anti_pattern', 'await-swallows-err')",
        (),
    )
    .await
    .expect("anti_pattern node");

    let node = "entity:ap".to_owned();
    let citation = "citation:p/surreal".to_owned();

    merge_node_into_citation(&db, &node, &citation)
        .await
        .expect("merge edge");

    let cited = citations_cited_by(&db, &node).await.expect("forward walk");
    assert_eq!(
        cited,
        vec![citation.clone()],
        "the node reaches its merged citation in one query"
    );

    let mut still_there = db
        .exec_params("SELECT name FROM entities WHERE id = 'entity:ap'", ())
        .await
        .expect("entity still queryable");
    assert!(
        still_there.next().await.expect("next").is_some(),
        "the anti_pattern row is untouched — merge is an edge, not a move"
    );

    merge_node_into_citation(&db, &node, &citation)
        .await
        .expect("replayed merge");
    let mut edges = db
        .exec_params(
            "SELECT COUNT(*) AS n FROM entity_edges WHERE from_id = ?1 AND rel_type = 'cite' AND to_id = ?2",
            libsql::params![node, citation],
        )
        .await
        .expect("edge count");
    let row = edges.next().await.expect("next").expect("row");
    let n: i64 = row.get(0).expect("get");
    assert_eq!(
        n, 1,
        "replayed merge is idempotent — exactly one cite edge, no double-count"
    );
}

#[tokio::test]
async fn citations_for_nodes_dedupes_across_recalled_rows() {
    let db = seed().await;
    let project = "p".to_owned();
    upsert_citation(
        &db,
        &UpsertCitation {
            project,
            entry_key: "surreal",
            name: "DbStoreDB",
            metadata: vec![meta("records", "https://surrealdb.com/docs")],
        },
    )
    .await
    .expect("citation");
    let citation = "citation:p/surreal".to_owned();
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, created_at, updated_at) \
         VALUES ('p', 'decision', 'k1', 't', 'c', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("decision");
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, created_at, updated_at) \
         VALUES ('p', 'roadmap', 'k2', 't', 'c', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("roadmap");
    let decision = "memory:p/decision/k1".to_owned();
    let roadmap = "memory:p/roadmap/k2".to_owned();
    merge_node_into_citation(&db, &decision, &citation)
        .await
        .expect("merge decision");
    merge_node_into_citation(&db, &roadmap, &citation)
        .await
        .expect("merge roadmap");

    let cits = citations_for_nodes(&db, &[decision, roadmap])
        .await
        .expect("batch recall");
    assert_eq!(
        cits,
        vec![citation],
        "both recalled rows cite the same citation -> deduped to one"
    );

    let empty = citations_for_nodes(&db, &[]).await.expect("empty batch");
    assert!(empty.is_empty(), "no recalled nodes -> no citations");
}

#[tokio::test]
async fn reward_flows_along_cite_edges() {
    let db = seed().await;
    let project = "p".to_owned();
    upsert_citation(
        &db,
        &UpsertCitation {
            project,
            entry_key: "surreal",
            name: "DbStoreDB",
            metadata: vec![meta("records", "https://surrealdb.com/docs")],
        },
    )
    .await
    .expect("citation");
    let citation = "citation:p/surreal".to_owned();
    db.exec_params(
        "INSERT INTO memories \
         (project, category, entry_key, title, content, created_at, updated_at) \
         VALUES ('p', 'decision', 'k', 't', 'c', ?1, ?1)",
        libsql::params![crate::db::now_rfc3339()],
    )
    .await
    .expect("decision");
    let node = "memory:p/decision/k".to_owned();
    merge_node_into_citation(&db, &node, &citation)
        .await
        .expect("merge");

    let n = reward_citation_edges(&db, &citation, 0.5)
        .await
        .expect("reward");
    assert_eq!(n, 1, "the single cite edge into the citation is rewarded");

    let mut weights = db
        .exec_params(
            "SELECT weight FROM entity_edges WHERE rel_type = 'cite' AND to_id = ?1",
            libsql::params![citation.clone()],
        )
        .await
        .expect("weights");
    let row = weights.next().await.expect("next").expect("row");
    let w: f64 = row.get(0).expect("get");
    assert!(
        (w - 1.5).abs() < f64::EPSILON,
        "edge weight climbed from the merge default 1.0 by delta 0.5 (got {w})"
    );

    let unlinked = reward_citation_edges(&db, "citation:p/absent", 1.0)
        .await
        .expect("reward on unlinked citation");
    assert_eq!(
        unlinked, 0,
        "a citation with no cite edges rewards nothing, no error"
    );
}
