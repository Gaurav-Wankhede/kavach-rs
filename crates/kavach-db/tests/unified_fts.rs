//! Integration tests for FTS5 BM25 search on unified_nodes.
//!
//! These tests verify that:
//! 1. FTS5 virtual table is created correctly
//! 2. BM25 search returns relevant results
//! 3. Search works across all categories (roadmap, research, decision, etc.)
//! 4. Results are ranked by relevance

#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_db::graph::{populate_fts, populate_nodes};
use kavach_db::turso::TursoStore;
use kavach_db::open_memory;

/// Helper: execute a SQL statement (INSERT/UPDATE/DELETE) via query_with_params.
async fn exec_sql(db: &TursoStore, sql: &str) {
    db.query_with_params(sql, ())
        .await
        .unwrap_or_else(|e| panic!("SQL failed: {e}\nSQL: {sql}"));
}

/// Helper: query rows and collect into a Vec.
async fn query_rows(db: &TursoStore, sql: &str) -> Vec<Vec<libsql::Value>> {
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

#[tokio::test]
async fn test_fts5_index_created() {
    let db = open_memory().await.expect("memory db");

    // Insert test data
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Thumbnail CTR Scoring', 'Implement thumbnail CTR prediction module', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes and FTS
    populate_nodes(&db).await.expect("populate_nodes");
    populate_fts(&db).await.expect("populate_fts");

    // Verify FTS5 table exists and has data
    let rows = query_rows(&db, "SELECT COUNT(*) FROM unified_nodes_fts").await;
    assert_eq!(rows.len(), 1);
    if let libsql::Value::Integer(count) = &rows[0][0] {
        assert_eq!(*count, 1, "FTS5 index should have 1 entry");
    } else {
        panic!("Expected integer count");
    }
}

#[tokio::test]
async fn test_bm25_search_returns_relevant_results() {
    let db = open_memory().await.expect("memory db");

    // Insert test data with different content
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Thumbnail CTR Scoring', 'Implement thumbnail CTR prediction module for YouTube packaging', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'research', 'research-1', 'Video Retention Analysis', 'Analyze audience retention patterns across 100 videos', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'decision', 'decision-1', 'Use Turso over SurrealDB', 'Chose Turso for embedded vector search', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes and FTS
    populate_nodes(&db).await.expect("populate_nodes");
    populate_fts(&db).await.expect("populate_fts");

    // Search for "thumbnail" - should return card-1
    let rows = query_rows(
        &db,
        "SELECT n.id, n.title, n.category
         FROM unified_nodes_fts fts
         JOIN unified_nodes n ON n.rowid = fts.rowid
         WHERE unified_nodes_fts MATCH 'thumbnail'
         ORDER BY fts.rank",
    )
    .await;

    assert!(!rows.is_empty(), "Should find results for 'thumbnail'");
    let first_result = &rows[0];
    if let libsql::Value::Text(id) = &first_result[0] {
        assert_eq!(id, "test-proj/roadmap/card-1");
    } else {
        panic!("Expected text id");
    }
}

#[tokio::test]
async fn test_bm25_search_ranks_by_relevance() {
    let db = open_memory().await.expect("memory db");

    // Insert test data with varying relevance
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Thumbnail Module', 'thumbnail thumbnail thumbnail', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-2', 'Other Module', 'thumbnail mentioned once', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes and FTS
    populate_nodes(&db).await.expect("populate_nodes");
    populate_fts(&db).await.expect("populate_fts");

    // Search for "thumbnail" - card-1 should rank first (more mentions)
    let rows = query_rows(
        &db,
        "SELECT n.id, n.title
         FROM unified_nodes_fts fts
         JOIN unified_nodes n ON n.rowid = fts.rowid
         WHERE unified_nodes_fts MATCH 'thumbnail'
         ORDER BY fts.rank",
    )
    .await;

    assert_eq!(rows.len(), 2, "Should find both cards");
    if let libsql::Value::Text(id) = &rows[0][0] {
        assert_eq!(id, "test-proj/roadmap/card-1", "More relevant should rank first");
    } else {
        panic!("Expected text id");
    }
}

#[tokio::test]
async fn test_bm25_search_across_categories() {
    let db = open_memory().await.expect("memory db");

    // Insert test data in different categories
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'roadmap', 'card-1', 'Vector Search', 'Implement vector similarity search', 'active', 'todo', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'research', 'research-1', 'Vector Embeddings', 'Study vector embedding models', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;
    exec_sql(
        &db,
        "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, tags, created_at, updated_at)
         VALUES ('test-proj', 'decision', 'decision-1', 'Vector Database', 'Choose vector database for storage', 'active', 'done', '[]', '2026-01-01', '2026-01-01')",
    )
    .await;

    // Populate unified_nodes and FTS
    populate_nodes(&db).await.expect("populate_nodes");
    populate_fts(&db).await.expect("populate_fts");

    // Search for "vector" - should return results from all categories
    let rows = query_rows(
        &db,
        "SELECT n.id, n.category
         FROM unified_nodes_fts fts
         JOIN unified_nodes n ON n.rowid = fts.rowid
         WHERE unified_nodes_fts MATCH 'vector'
         ORDER BY fts.rank",
    )
    .await;

    assert_eq!(rows.len(), 3, "Should find results from all categories");
    let categories: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            if let libsql::Value::Text(c) = &r[1] {
                Some(c.clone())
            } else {
                None
            }
        })
        .collect();
    assert!(categories.contains(&"roadmap".to_string()));
    assert!(categories.contains(&"research".to_string()));
    assert!(categories.contains(&"decision".to_string()));
}
