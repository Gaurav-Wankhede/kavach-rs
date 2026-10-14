//! Integration tests for unified graph migrations.
//!
//! These tests verify that:
//! 1. populate_nodes correctly copies memories → unified_nodes (read-only)
//! 2. extract_depends_on correctly parses DEPENDS_ON lines without modifying content
//! 3. create_cross_category_edges creates typed edges between categories
//! 4. build_transitive_closure builds correct ancestor/descendant paths
//! 5. Existing memories data is NEVER modified

#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_db::graph::{
    build_transitive_closure, create_cross_category_edges, extract_depends_on, populate_nodes,
};
use kavach_db::db::DbStore;
use kavach_db::open_memory;

/// Helper: execute a SQL statement (INSERT/UPDATE/DELETE) via query_with_params.
async fn exec_sql(db: &DbStore, sql: &str) {
    db.query_with_params(sql, ())
        .await
        .unwrap_or_else(|e| panic!("SQL failed: {e}\nSQL: {sql}"));
}

/// Helper: query rows and collect into a Vec.
async fn query_rows(db: &DbStore, sql: &str) -> Vec<Vec<libsql::Value>> {
    let mut rows = db.query_with_params(sql, ()).await.expect("query");
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.expect("row") {
        let mut cols = Vec::new();
        for i in 0..row.column_count() {
            cols.push(row.get_value(i).expect("value"));
        }
        out.push(cols);
    }
    out
}

/// Helper: get a single string value from a query.
async fn query_string(db: &DbStore, sql: &str) -> String {
    let rows = query_rows(db, sql).await;
    rows.first()
        .and_then(|r| r.first())
        .and_then(|v| match v {
            libsql::Value::Text(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Helper: get a single i64 value from a query.
async fn query_i64(db: &DbStore, sql: &str) -> i64 {
    let rows = query_rows(db, sql).await;
    rows.first()
        .and_then(|r| r.first())
        .and_then(|v| match v {
            libsql::Value::Integer(i) => Some(*i),
            _ => None,
        })
        .unwrap_or(0)
}

#[tokio::test]
async fn test_populate_nodes_copies_all_memories() {
    let db = open_memory().await.expect("memory db");

    // Insert test memories
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Test Card 1', 'Content 1', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'research', 'research-1', 'Test Research', 'Research content', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes
    let count = populate_nodes(&db).await.expect("populate_nodes");
    assert_eq!(count, 2, "Should copy 2 memories to unified_nodes");

    // Verify unified_nodes has correct data
    let ids = query_rows(&db, "SELECT id FROM unified_nodes ORDER BY id").await;
    let ids: Vec<String> = ids
        .iter()
        .filter_map(|r| {
            if let libsql::Value::Text(s) = &r[0] {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(ids.contains(&"test-proj/roadmap/card-1".to_string()));
    assert!(ids.contains(&"test-proj/research/research-1".to_string()));
}

#[tokio::test]
async fn test_extract_depends_on_does_not_modify_content() {
    let db = open_memory().await.expect("memory db");

    // Insert a roadmap card with DEPENDS_ON line
    let content_with_dep = "Some content\nDEPENDS_ON: card-a, card-b\nMore content";
    exec_sql(
        &db,
        &format!(
            "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
             VALUES ('test-proj', 'roadmap', 'card-1', 'Test Card', '{}', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
            content_with_dep.replace('\'', "''")
        ),
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-a', 'Card A', 'Content A', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-b', 'Card B', 'Content B', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes first
    populate_nodes(&db).await.expect("populate_nodes");

    // Extract DEPENDS_ON edges
    let count = extract_depends_on(&db).await.expect("extract_depends_on");
    assert_eq!(count, 2, "Should extract 2 DEPENDS_ON edges");

    // Verify content is NOT modified
    let content = query_string(&db, "SELECT content FROM memories WHERE entry_key = 'card-1'").await;
    assert_eq!(content, content_with_dep, "Content must not be modified");

    // Verify edges were created
    let edges = query_rows(
        &db,
        "SELECT source_id, target_id, edge_type FROM unified_edges ORDER BY target_id",
    )
    .await;
    assert_eq!(edges.len(), 2);
    let has_edge = |src: &str, tgt: &str| {
        edges.iter().any(|r| {
            if let (libsql::Value::Text(s), libsql::Value::Text(t)) = (&r[0], &r[1]) {
                s == src && t == tgt
            } else {
                false
            }
        })
    };
    assert!(has_edge("test-proj/roadmap/card-1", "test-proj/roadmap/card-a"));
    assert!(has_edge("test-proj/roadmap/card-1", "test-proj/roadmap/card-b"));
}

#[tokio::test]
async fn test_cross_category_edges() {
    let db = open_memory().await.expect("memory db");

    // Insert research, mistake, decision, and roadmap entries
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'research', 'research-1', 'Research 1', 'Research content', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'decision', 'decision-1', 'Decision 1', 'Decision content', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'See research-1 and decision-1', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes
    populate_nodes(&db).await.expect("populate_nodes");

    // Create cross-category edges
    let count = create_cross_category_edges(&db).await.expect("cross_category");
    assert_eq!(count, 2, "Should create 2 cross-category edges");

    // Verify edges
    let edges = query_rows(
        &db,
        "SELECT source_id, target_id, edge_type, weight FROM unified_edges ORDER BY edge_type",
    )
    .await;
    assert_eq!(edges.len(), 2);
    let has_edge = |src: &str, tgt: &str, ty: &str, w: f64| {
        edges.iter().any(|r| {
            if let (
                libsql::Value::Text(s),
                libsql::Value::Text(t),
                libsql::Value::Text(ty2),
                libsql::Value::Real(w2),
            ) = (&r[0], &r[1], &r[2], &r[3])
            {
                s == src && t == tgt && ty2 == ty && (w2 - w).abs() < 0.01
            } else {
                false
            }
        })
    };
    assert!(has_edge("test-proj/research/research-1", "test-proj/roadmap/card-1", "informs", 0.9));
    assert!(has_edge("test-proj/decision/decision-1", "test-proj/roadmap/card-1", "justifies", 0.7));
}

#[tokio::test]
async fn test_transitive_closure() {
    let db = open_memory().await.expect("memory db");

    // Insert a chain: A → B → C → D
    for (key, deps) in [
        ("card-a", ""),
        ("card-b", "card-a"),
        ("card-c", "card-b"),
        ("card-d", "card-c"),
    ] {
        let content = if deps.is_empty() {
            "Content".to_string()
        } else {
            format!("DEPENDS_ON: {}", deps)
        };
        exec_sql(
            &db,
            &format!(
                "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
                 VALUES ('test-proj', 'roadmap', '{}', '{}', '{}', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
                key, key, content
            ),
        )
        .await;
    }

    // Populate and extract
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");

    // Build transitive closure
    let count = build_transitive_closure(&db).await.expect("build_closure");
    assert!(count >= 3, "Should have at least 3 closure entries");

    // Verify: card-a should have 3 descendants (card-b, card-c, card-d)
    let descendants = query_rows(
        &db,
        "SELECT descendant_id, depth FROM unified_closure WHERE ancestor_id = 'test-proj/roadmap/card-a' ORDER BY depth",
    )
    .await;
    assert_eq!(descendants.len(), 3, "card-a should have 3 descendants");
    let get_depth = |r: &Vec<libsql::Value>| -> i64 {
        if let libsql::Value::Integer(i) = &r[1] {
            *i
        } else {
            0
        }
    };
    assert_eq!(get_depth(&descendants[0]), 1, "card-b is direct (depth 1)");
    assert_eq!(get_depth(&descendants[1]), 2, "card-c is transitive (depth 2)");
    assert_eq!(get_depth(&descendants[2]), 3, "card-d is transitive (depth 3)");
}

#[tokio::test]
async fn test_existing_data_never_modified() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    let original_content = "Original content\nDEPENDS_ON: card-a";
    exec_sql(
        &db,
        &format!(
            "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
             VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', '{}', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
            original_content.replace('\'', "''")
        ),
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-a', 'Card A', 'Content A', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");

    // Verify memories table is unchanged
    let content = query_string(&db, "SELECT content FROM memories WHERE entry_key = 'card-1'").await;
    assert_eq!(content, original_content, "memories.content must not be modified");

    // Verify edges table is still empty (we use unified_edges, not edges)
    let edge_count = query_i64(&db, "SELECT COUNT(*) FROM edges").await;
    assert_eq!(edge_count, 0, "edges table must remain empty");

    // Verify graph_nodes table is still empty (we use unified_nodes)
    let node_count = query_i64(&db, "SELECT COUNT(*) FROM graph_nodes").await;
    assert_eq!(node_count, 0, "graph_nodes table must remain empty");
}
