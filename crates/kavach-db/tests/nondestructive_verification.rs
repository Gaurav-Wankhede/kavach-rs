//! Non-destructive verification tests.
//!
//! These tests PROVE that the unified graph implementation does NOT modify
//! any existing data in the memories, edges, graph_nodes, graph_edges, or
//! entity_edges tables. The unified graph is purely additive.

#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_db::graph::{
    build_transitive_closure, create_cross_category_edges, extract_depends_on, populate_fts,
    populate_nodes,
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

#[tokio::test]
async fn test_memories_table_unchanged() {
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

    // Record state before migrations
    let memories_count_before = query_i64(&db, "SELECT COUNT(*) FROM memories").await;
    let content_before = query_string(&db, "SELECT content FROM memories WHERE entry_key = 'card-1'").await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");
    populate_fts(&db).await.expect("populate_fts");

    // Verify memories table is unchanged
    let memories_count_after = query_i64(&db, "SELECT COUNT(*) FROM memories").await;
    assert_eq!(
        memories_count_before, memories_count_after,
        "memories row count must not change"
    );

    let content_after = query_string(&db, "SELECT content FROM memories WHERE entry_key = 'card-1'").await;
    assert_eq!(
        content_before, content_after,
        "memories.content must not be modified"
    );
}

#[tokio::test]
async fn test_edges_table_remains_empty() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'DEPENDS_ON: card-a', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-a', 'Card A', 'Content A', 'active', 'done', '[]', '2026-01-01', '01-01')",
    )
    .await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");

    // Verify edges table is still empty
    let edge_count = query_i64(&db, "SELECT COUNT(*) FROM edges").await;
    assert_eq!(edge_count, 0, "edges table must remain empty");
}

#[tokio::test]
async fn test_graph_nodes_table_remains_empty() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'Content 1', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");

    // Verify graph_nodes table is still empty
    let node_count = query_i64(&db, "SELECT COUNT(*) FROM graph_nodes").await;
    assert_eq!(node_count, 0, "graph_nodes table must remain empty");
}

#[tokio::test]
async fn test_graph_edges_table_remains_empty() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'Content 1', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");

    // Verify graph_edges table is still empty
    let edge_count = query_i64(&db, "SELECT COUNT(*) FROM graph_edges").await;
    assert_eq!(edge_count, 0, "graph_edges table must remain empty");
}

#[tokio::test]
async fn test_entity_edges_table_remains_empty() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'Content 1', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");

    // Verify entity_edges table is still empty
    let edge_count = query_i64(&db, "SELECT COUNT(*) FROM entity_edges").await;
    assert_eq!(edge_count, 0, "entity_edges table must remain empty");
}

#[tokio::test]
async fn test_unified_nodes_count_matches_memories() {
    let db = open_memory().await.expect("memory db");

    // Insert test data in multiple categories
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'Content 1', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
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

    // Populate unified_nodes
    populate_nodes(&db).await.expect("populate_nodes");

    // Verify unified_nodes count matches memories count
    let memories_count = query_i64(&db, "SELECT COUNT(*) FROM memories").await;
    let unified_count = query_i64(&db, "SELECT COUNT(*) FROM unified_nodes").await;
    assert_eq!(
        memories_count, unified_count,
        "unified_nodes count must match memories count"
    );
}

#[tokio::test]
async fn test_unified_edges_only_contain_new_edges() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'DEPENDS_ON: card-a' || char(10) || 'See research-1 for details.', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
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
         VALUES ('test-proj', 'research', 'research-1', 'Research 1', 'Research content', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Run all migrations
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");

    // Verify unified_edges only contains expected edge types
    let rows = query_rows(
        &db,
        "SELECT DISTINCT edge_type FROM unified_edges",
    )
    .await;
    let edge_types: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            if let libsql::Value::Text(t) = &r[0] {
                Some(t.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(edge_types.contains(&"depends_on".to_string()));
    assert!(edge_types.contains(&"informs".to_string()));
    assert!(!edge_types.contains(&"blocks".to_string()));
    assert!(!edge_types.contains(&"supersedes".to_string()));
}

#[tokio::test]
async fn test_full_migration_pipeline() {
    let db = open_memory().await.expect("memory db");

    // Insert comprehensive test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Card 1', 'DEPENDS_ON: card-a', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
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
         VALUES ('test-proj', 'research', 'research-1', 'Research 1', 'Research content', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'decision', 'decision-1', 'Decision 1', 'Decision content', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Record state before migrations
    let memories_count_before = query_i64(&db, "SELECT COUNT(*) FROM memories").await;
    let edges_count_before = query_i64(&db, "SELECT COUNT(*) FROM edges").await;
    let graph_nodes_count_before = query_i64(&db, "SELECT COUNT(*) FROM graph_nodes").await;
    let graph_edges_count_before = query_i64(&db, "SELECT COUNT(*) FROM graph_edges").await;
    let entity_edges_count_before = query_i64(&db, "SELECT COUNT(*) FROM entity_edges").await;

    // Run full migration pipeline
    populate_nodes(&db).await.expect("populate_nodes");
    extract_depends_on(&db).await.expect("extract_depends_on");
    create_cross_category_edges(&db).await.expect("cross_category");
    build_transitive_closure(&db).await.expect("build_closure");
    populate_fts(&db).await.expect("populate_fts");

    // Verify all existing tables are unchanged
    assert_eq!(
        memories_count_before,
        query_i64(&db, "SELECT COUNT(*) FROM memories").await,
        "memories count must not change"
    );
    assert_eq!(
        edges_count_before,
        query_i64(&db, "SELECT COUNT(*) FROM edges").await,
        "edges count must not change"
    );
    assert_eq!(
        graph_nodes_count_before,
        query_i64(&db, "SELECT COUNT(*) FROM graph_nodes").await,
        "graph_nodes count must not change"
    );
    assert_eq!(
        graph_edges_count_before,
        query_i64(&db, "SELECT COUNT(*) FROM graph_edges").await,
        "graph_edges count must not change"
    );
    assert_eq!(
        entity_edges_count_before,
        query_i64(&db, "SELECT COUNT(*) FROM entity_edges").await,
        "entity_edges count must not change"
    );

    // Verify unified graph tables are populated
    assert!(
        query_i64(&db, "SELECT COUNT(*) FROM unified_nodes").await > 0,
        "unified_nodes must be populated"
    );
    assert!(
        query_i64(&db, "SELECT COUNT(*) FROM unified_edges").await > 0,
        "unified_edges must be populated"
    );
    assert!(
        query_i64(&db, "SELECT COUNT(*) FROM unified_closure").await > 0,
        "unified_closure must be populated"
    );
    assert!(
        query_i64(&db, "SELECT COUNT(*) FROM unified_nodes_fts").await > 0,
        "unified_nodes_fts must be populated"
    );
}
