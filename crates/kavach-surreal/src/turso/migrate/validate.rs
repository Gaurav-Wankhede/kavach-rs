//! Validation pass verifying entity-edge integrity, count parity, and FTS sync.
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
    let edges = store.all_edges_raw().await.map_err(|e| format!("read edges: {e}"))?;
    report.edge_count = edges.len();
    for (from, rel, to) in &edges {
        for endpoint in [from, to] {
            if store.get_entity(endpoint).await.ok().flatten().is_none() {
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
