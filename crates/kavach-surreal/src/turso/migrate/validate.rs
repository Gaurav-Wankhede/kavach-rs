//! Validation pass verifying entity-edge integrity, count parity, and FTS sync.
use std::collections::HashSet;

use super::types::{SourceCounts, ValidateReport};
use crate::turso::TursoStore;
use crate::turso::entries::EntryStore;
use crate::turso::graph::GraphStore;

/// Validate a populated TursoStore against source counts and graph integrity rules.
#[expect(clippy::missing_errors_doc, reason = "comment_noise_guard single-line ceiling")]
pub async fn validate_store(
    store: &TursoStore,
    expected: &SourceCounts,
) -> Result<ValidateReport, String> {
    let mut report = ValidateReport::default();
    let mut is_clean = true;
    let valid_nodes = load_valid_nodes(store).await?;
    let edges = store.all_edges_raw().await.map_err(|e| format!("read edges: {e}"))?;
    report.edge_count = edges.len();
    for (from, rel, to) in &edges {
        for endpoint in [from, to] {
            if !valid_nodes.contains(endpoint) {
                report.dangling_edges.push(format!("{from} -{rel}-> {to} (missing {endpoint})"));
                is_clean = false;
            }
        }
    }
    let memories = store.list_all().await.map_err(|e| format!("list memories: {e}"))?;
    report.memory_count = memories.len();
    if report.memory_count != expected.memory {
        is_clean = false;
    }
    let entities = store.list_entities(None).await.map_err(|e| format!("list entities: {e}"))?;
    report.entity_count = entities.len();
    if report.entity_count != expected.entity {
        is_clean = false;
    }
    if report.edge_count != expected.edge {
        is_clean = false;
    }
    let fts_count = get_fts_count(store).await.unwrap_or(0);
    report.fts_matched = fts_count == report.memory_count;
    if !report.fts_matched {
        is_clean = false;
    }
    report.is_clean = is_clean;
    Ok(report)
}

/// Load all valid endpoint IDs from entities, memories, and projects tables.
async fn load_valid_nodes(store: &TursoStore) -> Result<HashSet<String>, String> {
    let mut nodes = HashSet::new();
    let mut ent_rows = store.query("SELECT id FROM entities", ()).await.map_err(|e| format!("query entities: {e}"))?;
    while let Some(row) = ent_rows.next().await.map_err(|e| format!("entity row: {e}"))? {
        let id: String = row.get(0).map_err(|e| format!("entity id: {e}"))?;
        if let Some(stripped) = id.strip_prefix("entity:") {
            nodes.insert(stripped.to_owned());
        } else {
            nodes.insert(format!("entity:{id}"));
        }
        nodes.insert(id);
    }
    let mut mem_rows = store.query("SELECT project, category, entry_key FROM memories", ()).await.map_err(|e| format!("query memories: {e}"))?;
    while let Some(row) = mem_rows.next().await.map_err(|e| format!("memory row: {e}"))? {
        let proj: String = row.get(0).map_err(|e| format!("mem proj: {e}"))?;
        let cat: String = row.get(1).map_err(|e| format!("mem cat: {e}"))?;
        let key: String = row.get(2).map_err(|e| format!("mem key: {e}"))?;
        nodes.insert(format!("{cat}:{proj}:{key}"));
        nodes.insert(format!("{proj}/{cat}/{key}"));
        nodes.insert(format!("{cat}:{key}"));
    }
    let mut proj_rows = store.query("SELECT slug FROM projects", ()).await.map_err(|e| format!("query projects: {e}"))?;
    while let Some(row) = proj_rows.next().await.map_err(|e| format!("proj row: {e}"))? {
        let slug: String = row.get(0).map_err(|e| format!("proj slug: {e}"))?;
        nodes.insert(format!("project:{slug}"));
        nodes.insert(format!("entity:project:{slug}"));
        nodes.insert(slug);
    }
    Ok(nodes)
}

/// Fetch row count from memories_fts virtual table.
async fn get_fts_count(store: &TursoStore) -> Result<usize, String> {
    let mut rows = store.query("SELECT count(*) FROM memories_fts", ()).await.map_err(|e| format!("fts count query: {e}"))?;
    if let Some(row) = rows.next().await.map_err(|e| format!("fts count row: {e}"))? {
        let count: i64 = row.get(0).map_err(|e| format!("fts count col: {e}"))?;
        usize::try_from(count).map_err(|e| format!("fts count negative: {e}"))
    } else {
        Ok(0)
    }
}
