//! Regression proof for the Knowledge Graph edge-resolution path.
//!
//! The KG view lists entities of a type, then must draw only the edges whose
//! BOTH endpoints are in that visible set — an edge to an off-canvas node would
//! render a line to nowhere. `list_edges_among` builds its per-relation queries
//! as `&str`, so `cargo check` cannot prove they parse or filter correctly; only
//! execution against a real DB can. This seeds entities + edges and asserts the
//! filter keeps in-set edges, drops boundary-crossing ones, and keys endpoints
//! the same way the renderer does.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_surreal::{
    graph_list_edges_among, graph_relate_dynamic, graph_upsert_entity, open_memory,
};

#[tokio::test]
async fn list_edges_among_keeps_intra_set_drops_boundary() {
    let db = open_memory().await.expect("memory db");
    kavach_surreal::apply_schema(&db).await.expect("schema");

    // Three concepts in the visible set; one extra outside it.
    let a = graph_upsert_entity(&db, "concept", "alpha")
        .await
        .expect("a");
    let b = graph_upsert_entity(&db, "concept", "beta")
        .await
        .expect("b");
    let c = graph_upsert_entity(&db, "concept", "gamma")
        .await
        .expect("c");
    let outside = graph_upsert_entity(&db, "concept", "outside")
        .await
        .expect("outside");

    // a->b and b->c are intra-set; c->outside crosses the visible boundary.
    graph_relate_dynamic(&db, &a, &b, "is_a", 1.0)
        .await
        .expect("a->b");
    graph_relate_dynamic(&db, &b, &c, "part_of", 1.0)
        .await
        .expect("b->c");
    graph_relate_dynamic(&db, &c, &outside, "references", 1.0)
        .await
        .expect("c->outside");

    let set = vec![a.clone(), b.clone(), c.clone()];
    let edges = graph_list_edges_among(&db, &set).await.expect("edges");

    assert_eq!(edges.len(), 2, "only the two intra-set edges survive");

    // Endpoints are keyed as format!("{id:?}") — the exact form the KG renderer
    // and the node-id builder in graph_fetch_impl use, so edges resolve to nodes.
    let key_a = format!("{a:?}");
    let key_b = format!("{b:?}");
    let key_c = format!("{c:?}");
    let key_outside = format!("{outside:?}");

    let has = |from: &str, to: &str, rel: &str| {
        edges
            .iter()
            .any(|e| e.from == from && e.to == to && e.rel_type == rel)
    };
    assert!(has(&key_a, &key_b, "is_a"), "a->b is_a present");
    assert!(has(&key_b, &key_c, "part_of"), "b->c part_of present");
    assert!(
        !edges.iter().any(|e| e.to == key_outside),
        "no edge to the off-canvas node"
    );
}

#[tokio::test]
async fn list_edges_among_short_sets_yield_nothing() {
    // < 2 nodes can have no internal edge; the fn must short-circuit, not query.
    let db = open_memory().await.expect("memory db");
    kavach_surreal::apply_schema(&db).await.expect("schema");
    let lone = graph_upsert_entity(&db, "concept", "solo")
        .await
        .expect("solo");
    assert!(
        graph_list_edges_among(&db, &[lone])
            .await
            .expect("one")
            .is_empty(),
        "single node -> no edges"
    );
    assert!(
        graph_list_edges_among(&db, &[])
            .await
            .expect("zero")
            .is_empty(),
        "empty set -> no edges"
    );
}
