//! GraphStore tests — Phase 3 acceptance (PRD §4 Phase 3).

use super::DbStore;
use super::graph::{
    EdgeRow, GraphEntity, GraphStore, render_entity_fixture, render_entity_line,
    render_named_lines, validate_rel,
};

fn entity(id: &str, etype: &str, name: &str) -> GraphEntity {
    GraphEntity {
        id: id.to_owned(),
        entity_type: etype.to_owned(),
        name: name.to_owned(),
        properties: None,
        content_hash: None,
        project: None,
    }
}

#[tokio::test]
async fn entity_upsert_get_roundtrip() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = DbStore::open(dir.path().join("g1.db")).await.unwrap();
    s.upsert_entity(&entity("e1", "concept", "reciprocal-rank-fusion"))
        .await
        .unwrap();
    let got = s.get_entity("e1").await.unwrap().unwrap();
    assert_eq!(got.entity_type, "concept");
    assert_eq!(got.name, "reciprocal-rank-fusion");

    // Upsert again updates, not duplicates.
    let mut e2 = entity("e1", "concept", "rrf");
    e2.properties = Some(r#"{"alias":"RRF"}"#.into());
    s.upsert_entity(&e2).await.unwrap();
    let all = s.list_entities(None).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "rrf");

    // Type filter + find-by-name.
    assert_eq!(s.list_entities(Some("gate")).await.unwrap().len(), 0);
    assert!(s.find_entity("concept", "rrf").await.unwrap().is_some());
    assert!(s.find_entity("concept", "missing").await.unwrap().is_none());
}

#[tokio::test]
async fn edges_relate_traverse_delete() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = DbStore::open(dir.path().join("g2.db")).await.unwrap();
    for (id, etype, name) in [
        ("a", "roadmap", "card-a"),
        ("b", "decision", "decision-b"),
        ("c", "pattern", "pattern-c"),
        ("d", "session", "session-d"),
    ] {
        s.upsert_entity(&entity(id, etype, name)).await.unwrap();
    }

    // a -depends_on-> b -references-> c ; d isolated.
    s.relate_dynamic("a", "b", "depends_on", 1.0).await.unwrap();
    s.relate_dynamic("b", "c", "references", 2.0).await.unwrap();

    // Unknown rel rejected (typo-guard parity).
    assert!(
        s.relate_dynamic("a", "c", "depends_onn", 1.0)
            .await
            .is_err()
    );

    // Forward/backward hops.
    let fwd = s.related_forward("a").await.unwrap();
    assert_eq!(fwd.len(), 1);
    assert_eq!(fwd[0].to, "b");
    let back = s.related_backward("c").await.unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].from, "b");
    assert!((back[0].weight - 2.0).abs() < f64::EPSILON);

    // Closure reaches c through b but not the isolated d.
    let closure = s.traverse_closure("a", 4).await.unwrap();
    assert!(closure.contains(&"b".to_owned()));
    assert!(closure.contains(&"c".to_owned()));
    assert!(!closure.contains(&"d".to_owned()));

    // Delete edge removes it.
    assert_eq!(s.delete_edge("b", "references", "c").await.unwrap(), 1);
    let back_after = s.related_backward("c").await.unwrap();
    assert!(back_after.is_empty());
}

#[test]
fn fixture_renderer_matches_golden_shape() {
    let e = GraphEntity {
        id: "abc123".into(),
        entity_type: "skill".into(),
        name: "loop-engineering".into(),
        properties: Some(r#"{"level":2}"#.into()),
        content_hash: None,
        project: None,
    };
    let rendered = render_entity_fixture(&e);
    assert_eq!(
        rendered,
        r#"[skill] loop-engineering (id=String { table: Table("entity"), key: String("abc123") }) {"level":2}"#
    );
}

#[test]
fn rel_allowlist_guards_typos() {
    assert!(validate_rel("depends_on").is_ok());
    assert!(validate_rel("deends_on").is_err());
}

#[tokio::test]
async fn get_related_resolves_targets() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = DbStore::open(dir.path().join("g3.db")).await.unwrap();
    s.upsert_entity(&entity("n1", "decision", "source-decision"))
        .await
        .unwrap();
    s.upsert_entity(&entity("n2", "gate", "suppression_swap"))
        .await
        .unwrap();
    s.relate_dynamic("n1", "n2", "references", 3.0)
        .await
        .unwrap();

    let related = s.get_related("n1", 50).await.unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].rel_type, "references");
    assert!((related[0].weight - 3.0).abs() < f64::EPSILON);
    // Target entity fully resolved (the JOIN did its job).
    assert_eq!(related[0].target.id, "n2");
    assert_eq!(related[0].target.entity_type, "gate");
    assert_eq!(related[0].target.name, "suppression_swap");
}

#[tokio::test]
async fn list_edges_among_filters_to_set_and_short_circuits() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = DbStore::open(dir.path().join("g4.db")).await.unwrap();
    for id in ["a", "b", "c"] {
        s.upsert_entity(&entity(id, "file", &format!("f-{id}")))
            .await
            .unwrap();
    }
    s.relate_dynamic("a", "b", "mentions", 1.0).await.unwrap();
    // Boundary edge: c -> external node must be dropped.
    s.relate_dynamic("c", "external-x", "mentions", 1.0)
        .await
        .unwrap();

    // Short-set short-circuit: <2 nodes yields nothing without a query.
    let none = s.list_edges_among(&["a".to_owned()]).await.unwrap();
    assert!(none.is_empty());

    let rows: Vec<EdgeRow> = s
        .list_edges_among(&["a".to_owned(), "b".to_owned(), "c".to_owned()])
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].from.as_str(),
            rows[0].rel_type.as_str(),
            rows[0].to.as_str()
        ),
        ("a", "mentions", "b")
    );
}

#[test]
fn named_renderer_parity_with_rpc_shapes() {
    let src = GraphEntity {
        id: "k1".into(),
        entity_type: "roadmap".into(),
        name: "unit.migration".into(),
        properties: None,
        content_hash: None,
        project: None,
    };
    let target = GraphEntity {
        id: "k2".into(),
        entity_type: "decision".into(),
        name: "use-libsql".into(),
        properties: None,
        content_hash: None,
        project: None,
    };
    let related = vec![super::graph::RelatedRow {
        rel_type: "depends_on".into(),
        weight: 1.0,
        target,
    }];
    let lines = render_named_lines(&src, &related);
    assert_eq!(
        lines[0],
        r#"[roadmap] unit.migration (id=String { table: Table("entity"), key: String("k1") })"#
    );
    assert_eq!(lines[1], "  ──depends_on──► [decision] use-libsql");

    // Empty relations render the sentinel line.
    let mut empty_target = src;
    empty_target.properties = Some("{}".into());
    let no_edges = render_named_lines(&empty_target, &[]);
    assert_eq!(no_edges[1], "  (no edges)");
}

#[test]
fn list_line_matches_fixture_bytes() {
    let e = GraphEntity {
        id: "01abc".into(),
        entity_type: "session".into(),
        name: "auto-49431".into(),
        properties: None,
        content_hash: None,
        project: None,
    };
    assert_eq!(
        render_entity_line(&e),
        r#"[session] auto-49431 (id=String { table: Table("entity"), key: String("01abc") })"#
    );
}
