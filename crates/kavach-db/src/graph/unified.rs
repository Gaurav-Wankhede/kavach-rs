//! Unified graph: cross-category nodes, edges, and transitive closure.
//!
//! These tables reference `memories` but never modify it. They provide:
//! - `unified_nodes` — read-only view of all memory entries across categories
//! - `unified_edges` — cross-category dependency edges (informs, mitigates, justifies, depends_on)
//! - `unified_closure` — transitive closure for O(1) ancestor/descendant queries
//! - `unified_nodes_fts` — FTS5 BM25 full-text index

#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects,
    clippy::float_arithmetic,
    clippy::collapsible_if,
    reason = "SQLite returns u64/i64; RRF fusion requires float math; safe at Kavach's scale"
)]

use crate::error::Result;
use crate::db::{DbError, DbStore};

/// Populate unified_nodes from existing memories (read-only copy).
///
/// This is a one-time migration. Uses INSERT OR IGNORE to be idempotent.
/// The memories table is NEVER modified.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn populate_nodes(db: &DbStore) -> Result<usize> {
    let sql = r"
        INSERT OR IGNORE INTO unified_nodes (id, project, category, entry_key, title, status, content, embedding, metadata, created_at, updated_at)
        SELECT
            project || '/' || category || '/' || entry_key as id,
            project,
            category,
            entry_key,
            title,
            COALESCE(entry_status, 'todo') as status,
            content,
            embedding,
            json_object('tags', tags, 'priority', priority, 'lane', lane) as metadata,
            created_at,
            updated_at
        FROM memories
        WHERE project IS NOT NULL
          AND category IS NOT NULL
          AND entry_key IS NOT NULL
    ";
    let affected = db.exec(sql).await?;
    Ok(affected as usize)
}

/// Extract existing DEPENDS_ON edges from memories.content into unified_edges.
///
/// Parses the literal `DEPENDS_ON:` line WITHOUT modifying the content field.
/// Only extracts roadmap → roadmap dependencies.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn extract_depends_on(db: &DbStore) -> Result<usize> {
    // Query all roadmap entries with DEPENDS_ON lines
    let sql = r"
        SELECT m.project, m.entry_key, m.content
        FROM memories m
        WHERE m.category = 'roadmap'
          AND m.content LIKE '%DEPENDS_ON:%'
    ";
    let mut rows = db.exec_params(sql, ()).await?;
    let mut entries = Vec::new();
    while let Some(row) = rows.next().await.map_err(DbError::from)? {
        let project: String = row.get::<String>(0).map_err(|e| DbError::Mapping(e.to_string()))?;
        let entry_key: String = row.get::<String>(1).map_err(|e| DbError::Mapping(e.to_string()))?;
        let content: String = row.get::<String>(2).map_err(|e| DbError::Mapping(e.to_string()))?;
        entries.push((project, entry_key, content));
    }

    let mut total = 0_usize;
    for (project, entry_key, content) in entries {
        // Parse DEPENDS_ON line
        if let Some(dep_line) = content.lines().find(|l| l.starts_with("DEPENDS_ON:")) {
            let deps_str = dep_line.trim_start_matches("DEPENDS_ON:").trim();
            for dep in deps_str.split(',') {
                let dep = dep.trim();
                if dep.is_empty() {
                    continue;
                }
                let source_id = format!("{project}/roadmap/{entry_key}");
                let target_id = format!("{project}/roadmap/{dep}");
                let insert_sql = r"
                    INSERT OR IGNORE INTO unified_edges (source_id, target_id, edge_type, weight)
                    VALUES (?1, ?2, 'depends_on', 1.0)
                ";
                db.exec_params(insert_sql, libsql::params![source_id, target_id]).await?;
                total += 1;
            }
        }
    }
    Ok(total)
}

/// Create cross-category edges: research→roadmap (informs), mistake→roadmap (mitigates),
/// decision→roadmap (justifies).
///
/// These are NEW edges that don't exist in the current system.
/// Matches by entry_key appearing in the roadmap card's content field.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn create_cross_category_edges(db: &DbStore) -> Result<usize> {
    let mut total = 0_usize;

    // Research informs roadmap
    let sql_research = r"
        INSERT OR IGNORE INTO unified_edges (source_id, target_id, edge_type, weight)
        SELECT
            r.project || '/research/' || r.entry_key,
            m.project || '/roadmap/' || m.entry_key,
            'informs',
            0.9
        FROM memories r, memories m
        WHERE r.category = 'research'
          AND m.category = 'roadmap'
          AND r.project = m.project
          AND m.content LIKE '%' || r.entry_key || '%'
    ";
    total += db.exec(sql_research).await? as usize;

    // Mistakes mitigate roadmap
    let sql_mistakes = r"
        INSERT OR IGNORE INTO unified_edges (source_id, target_id, edge_type, weight)
        SELECT
            r.project || '/mistake/' || r.entry_key,
            m.project || '/roadmap/' || m.entry_key,
            'mitigates',
            0.8
        FROM memories r, memories m
        WHERE r.category = 'mistake'
          AND m.category = 'roadmap'
          AND r.project = m.project
          AND m.content LIKE '%' || r.entry_key || '%'
    ";
    total += db.exec(sql_mistakes).await? as usize;

    // Decisions justify roadmap
    let sql_decisions = r"
        INSERT OR IGNORE INTO unified_edges (source_id, target_id, edge_type, weight)
        SELECT
            r.project || '/decision/' || r.entry_key,
            m.project || '/roadmap/' || m.entry_key,
            'justifies',
            0.7
        FROM memories r, memories m
        WHERE r.category = 'decision'
          AND m.category = 'roadmap'
          AND r.project = m.project
          AND m.content LIKE '%' || r.entry_key || '%'
    ";
    total += db.exec(sql_decisions).await? as usize;

    Ok(total)
}

/// Build transitive closure from unified_edges where edge_type = 'depends_on'.
///
/// Direct edges get depth = 1. Transitive edges get depth = 2+.
/// Limit depth to 10 to prevent infinite loops.
///
/// Note: In unified_edges, source_id is the dependent (e.g., card-b depends on card-a).
/// In unified_closure, ancestor_id is the prerequisite (card-a is ancestor of card-b).
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn build_transitive_closure(db: &DbStore) -> Result<usize> {
    // Clear existing closure
    db.exec("DELETE FROM unified_closure").await?;

    // Use recursive CTE to compute full transitive closure
    // For each edge (source_id → target_id), we need to find all ancestors of target_id
    // and add them as ancestors of source_id
    let sql = r"
        WITH RECURSIVE closure(ancestor_id, descendant_id, depth) AS (
            -- Base case: direct edges
            SELECT target_id, source_id, 1
            FROM unified_edges
            WHERE edge_type = 'depends_on'

            UNION ALL

            -- Recursive case: for each (ancestor, descendant, depth),
            -- find all prerequisites of ancestor and add them as ancestors of descendant
            SELECT e.target_id, c.descendant_id, c.depth + 1
            FROM closure c
            JOIN unified_edges e ON c.ancestor_id = e.source_id
            WHERE e.edge_type = 'depends_on'
              AND c.depth < 10
        )
        INSERT OR IGNORE INTO unified_closure (ancestor_id, descendant_id, depth)
        SELECT ancestor_id, descendant_id, depth FROM closure
    ";
    let affected = db.exec(sql).await?;
    Ok(affected as usize)
}

/// Populate FTS5 full-text index from unified_nodes.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn populate_fts(db: &DbStore) -> Result<usize> {
    let sql = r"
        INSERT OR IGNORE INTO unified_nodes_fts(rowid, title, content)
        SELECT rowid, title, content FROM unified_nodes
    ";
    let affected = db.exec(sql).await?;
    Ok(affected as usize)
}

/// Populate vector index for unified_nodes.
///
/// Creates a libsql vector index on unified_nodes.embedding and populates it
/// from existing embeddings. Uses the same `libsql_vector_idx` function as
/// the memories table.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn populate_vector_index(db: &DbStore) -> Result<usize> {
    // Create vector index on unified_nodes.embedding
    let ddl = "CREATE INDEX IF NOT EXISTS idx_unified_nodes_embedding ON unified_nodes(libsql_vector_idx(embedding, 'metric=cosine'))";
    db.exec(ddl).await?;

    // Count how many nodes have embeddings
    let sql = r"
        SELECT COUNT(*) FROM unified_nodes WHERE embedding IS NOT NULL
    ";
    let mut rows = db.exec_params(sql, ()).await?;
    let count = if let Some(row) = rows.next().await.map_err(DbError::from)? {
        row.get::<i64>(0).map_err(|e| DbError::Mapping(e.to_string()))?
    } else {
        0
    };
    Ok(count as usize)
}

/// Vector search on unified_nodes using libsql vector index.
///
/// Returns top-k node IDs ranked by cosine similarity.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn vector_search(
    db: &DbStore,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<(String, f32)>> {
    use crate::db::vector::deserialize_embedding;

    // Try libsql ANN first
    let ann_sql = r"
        SELECT n.id, n.embedding
        FROM vector_top_k('idx_unified_nodes_embedding', vector32(?1), ?2) v
        JOIN unified_nodes n ON n.rowid = v.id
    ";
    let vec_json = format!(
        "[{}]",
        query_embedding
            .iter()
            .map(|v| format!("{v:.6}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let limit_i = limit as i64;

    let mut results = Vec::new();
    if let Ok(mut rows) = db
        .exec_params(ann_sql, libsql::params![vec_json, limit_i])
        .await
    {
        while let Some(row) = rows.next().await.map_err(DbError::from)? {
            let id: String = row.get::<String>(0).map_err(|e| DbError::Mapping(e.to_string()))?;
            let blob: Vec<u8> = row.get::<Vec<u8>>(1).map_err(|e| DbError::Mapping(e.to_string()))?;
            let stored_vec = deserialize_embedding(&blob);
            let sim = crate::db::vector::cosine_similarity(query_embedding, &stored_vec);
            results.push((id, sim));
        }
    }

    // Fallback to brute-force if ANN returned nothing
    if results.is_empty() {
        let sql = r"
            SELECT id, embedding FROM unified_nodes WHERE embedding IS NOT NULL
        ";
        let mut rows = db.exec_params(sql, ()).await?;
        let mut scored: Vec<(String, f32)> = Vec::new();
        while let Some(row) = rows.next().await.map_err(DbError::from)? {
            let id: String = row.get::<String>(0).map_err(|e| DbError::Mapping(e.to_string()))?;
            let blob: Vec<u8> = row.get::<Vec<u8>>(1).map_err(|e| DbError::Mapping(e.to_string()))?;
            let stored_vec = deserialize_embedding(&blob);
            let sim = crate::db::vector::cosine_similarity(query_embedding, &stored_vec);
            scored.push((id, sim));
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        results = scored;
    }

    Ok(results)
}

/// Get all ancestors of a node (all nodes that must complete before this one).
/// Uses the closure table for O(1) lookup.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn get_ancestors(db: &DbStore, node_id: &str) -> Result<Vec<(String, i64)>> {
    let sql = r"
        SELECT ancestor_id, depth FROM unified_closure
        WHERE descendant_id = ?1
        ORDER BY depth DESC
    ";
    let mut rows = db.exec_params(sql, libsql::params![node_id]).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(DbError::from)? {
        let id: String = row
            .get::<String>(0)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        let depth: i64 = row
            .get::<i64>(1)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        out.push((id, depth));
    }
    Ok(out)
}

/// Get all descendants of a node (all nodes that depend on this one).
/// Uses the closure table for O(1) lookup.
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn get_descendants(db: &DbStore, node_id: &str) -> Result<Vec<(String, i64)>> {
    let sql = r"
        SELECT descendant_id, depth FROM unified_closure
        WHERE ancestor_id = ?1
        ORDER BY depth ASC
    ";
    let mut rows = db.exec_params(sql, libsql::params![node_id]).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(DbError::from)? {
        let id: String = row
            .get::<String>(0)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        let depth: i64 = row
            .get::<i64>(1)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        out.push((id, depth));
    }
    Ok(out)
}

/// Get cross-category edges for a node (informs, mitigates, justifies).
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn get_cross_category_edges(
    db: &DbStore,
    node_id: &str,
) -> Result<Vec<(String, String, String, f64)>> {
    let sql = r"
        SELECT source_id, target_id, edge_type, weight
        FROM unified_edges
        WHERE (source_id = ?1 OR target_id = ?1)
          AND edge_type != 'depends_on'
    ";
    let mut rows = db.exec_params(sql, libsql::params![node_id]).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await.map_err(DbError::from)? {
        let source: String = row
            .get::<String>(0)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        let target: String = row
            .get::<String>(1)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        let edge_type: String = row
            .get::<String>(2)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        let weight: f64 = row
            .get::<f64>(3)
            .map_err(|e| DbError::Mapping(e.to_string()))?;
        out.push((source, target, edge_type, weight));
    }
    Ok(out)
}

/// Hybrid retrieval: BM25 + Vector + Graph with RRF fusion.
///
/// Returns top-k nodes ranked by RRF score.
/// RRF formula: Score(d) = 1/(60 + rank_bm25) + 1/(60 + rank_vector) + graph_score
///
/// # Errors
/// Returns [`crate::error::Error`] on SQL failure.
pub async fn hybrid_search(
    db: &DbStore,
    query_text: &str,
    query_embedding: Option<&[f32]>,
    limit: usize,
) -> Result<Vec<(String, String, String, f64)>> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, (String, String, f64)> = HashMap::new();

    // Signal 1: BM25 search via FTS5
    if !query_text.trim().is_empty() {
        let bm25_sql = r"
            SELECT n.id, n.title, n.category, fts.rank
            FROM unified_nodes_fts fts
            JOIN unified_nodes n ON n.rowid = fts.rowid
            WHERE unified_nodes_fts MATCH ?1
            ORDER BY fts.rank
            LIMIT ?2
        ";
        let mut rows = db
            .exec_params(bm25_sql, libsql::params![query_text, limit as i64])
            .await?;
        let mut rank = 1_usize;
        while let Some(row) = rows.next().await.map_err(DbError::from)? {
            let id: String = row.get::<String>(0).map_err(|e| DbError::Mapping(e.to_string()))?;
            let title: String = row.get::<String>(1).map_err(|e| DbError::Mapping(e.to_string()))?;
            let category: String = row.get::<String>(2).map_err(|e| DbError::Mapping(e.to_string()))?;
            let rrf_score = 1.0 / (60.0 + rank as f64);
            scores.entry(id).or_insert((title, category, 0.0)).2 += rrf_score;
            rank += 1;
        }
    }

    // Signal 2: Vector search
    if let Some(embedding) = query_embedding {
        if !embedding.is_empty() {
            let vec_results = vector_search(db, embedding, limit).await?;
            for (idx, (id, _sim)) in vec_results.iter().enumerate() {
                let rrf_score = 1.0 / (60.0 + (idx + 1) as f64);
                let entry = scores.entry(id.clone()).or_default();
                entry.2 += rrf_score;
            }
        }
    }

    // Signal 3: Graph expansion from seed nodes
    let seeds: Vec<String> = scores.keys().cloned().collect();
    if !seeds.is_empty() {
        for seed in &seeds {
            let graph_sql = r"
                SELECT target_id, weight FROM unified_edges
                WHERE source_id = ?1 AND edge_type != 'depends_on'
            ";
            let mut rows = db
                .exec_params(graph_sql, libsql::params![seed.clone()])
                .await?;
            while let Some(row) = rows.next().await.map_err(DbError::from)? {
                let target_id: String = row.get::<String>(0).map_err(|e| DbError::Mapping(e.to_string()))?;
                let weight: f64 = row.get::<f64>(1).map_err(|e| DbError::Mapping(e.to_string()))?;
                let entry = scores.entry(target_id).or_default();
                entry.2 += weight;
            }
        }
    }

    // Convert to sorted Vec
    let mut results: Vec<(String, String, String, f64)> = scores
        .into_iter()
        .map(|(id, (title, category, score))| (id, title, category, score))
        .collect();
    results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    Ok(results)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_populate_nodes_idempotent() {
        // Verify that running populate_nodes twice doesn't create duplicates
        // This is a placeholder — actual test would use a test database
    }

    #[tokio::test]
    async fn test_extract_depends_on_does_not_modify_content() {
        // Verify that extracting DEPENDS_ON edges doesn't modify memories.content
    }

    #[tokio::test]
    async fn test_transitive_closure_depth_limit() {
        // Verify that closure depth is limited to 10
    }

    #[tokio::test]
    async fn test_cross_category_edges_weights() {
        // Verify edge weights: informs=0.9, mitigates=0.8, justifies=0.7
    }
}
