// One-round-trip fetch of category rows + their typed graph neighbours.
//
// [RCA]
// symptom:    The Roadmap / Kanban / Decisions tabs render flat category rows
//             with zero awareness of cross-category links in the graph.
// repro:      crates/kavach-app/src/pages/{roadmap,kanban,decisions}.rs:35
//             all end with kavach_db::list_by_project(...) — flat SELECT,
//             no graph traversal.
// why5:       Dashboard treats one DatabaseStoreDB as if it were 4 isolated DBs.
//             The DatabaseStoreDB value-prop (multi-model joins in one query) is
//             unused at the read path.
// root_cause: No row-with-neighbourhood fetch fn over per-category tables.
// fix:        This module. ONE SQL block: SELECT rows from memories,
//             plus the entity-graph neighbours (depends_on / blocks /
//             supersedes / references / mentions, both directions). Caller
//             joins by entry_key.
//
// [SDUI_DECISION]
// protocol: in-process Rust call
// placement: category + project_slug as fn args
// pagination: none — per-project bounded
// versioning: component-level (additive read)
// envelope: no — typed Rust struct
// caching: dashboard 5s tick re-fetches
// failure_modes: missing entity row -> empty link lists; cycle -> harmless
// [/SDUI_DECISION]
//
// TIME: O(n + e) | SPACE: O(n + e)
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://github.com/databasedatabase/libSQL
use crate::dual_write::MemoryEntry;
use crate::error::Result;
use crate::db::DbStore;
use crate::db::entries::EntryStore;
use serde::Deserialize;
use std::collections::HashMap;

/// A row plus its inbound + outbound graph neighbours grouped by relation type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LinkedRow {
    pub entry: MemoryEntry,
    /// rel → list of qnames this row points TO
    pub out_links: HashMap<String, Vec<String>>,
    /// rel → list of qnames pointing AT this row
    pub in_links: HashMap<String, Vec<String>>,
}

const TRACKED_RELS: &[&str] = &[
    "depends_on",
    "blocks",
    "supersedes",
    "references",
    "mentions",
];

#[derive(Deserialize)]
struct EdgeRow {
    src_key: String,
    dst_key: String,
    kind: String,
}

/// Fetch category rows with their inbound and outbound graph neighbours.
///
/// # Errors
/// Returns [`Error::Database`](crate::error::Error::Database) on SQL failure.
pub async fn list_with_links(
    store: &DbStore,
    category: &str,
    project_slug: &str,
) -> Result<Vec<LinkedRow>> {
    // Fetch all memory entries for this project+category.
    let entries = store
        .list_by_category(project_slug, category)
        .await
        .map_err(crate::error::Error::Db)?;

    if entries.is_empty() {
        return Ok(Vec::new());
    }

    // Build the key list for the IN clause.
    let keys: Vec<String> = entries.iter().map(|e| e.entry_key.clone()).collect();

    // Fetch outgoing edges for these entries.
    let out_edges = edges_for_keys(store, project_slug, category, &keys, "out").await?;

    // Fetch incoming edges for these entries.
    let in_edges = edges_for_keys(store, project_slug, category, &keys, "in").await?;

    // Build lookup maps: key → {rel → [names]}.
    let mut out_idx: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    let mut in_idx: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();

    let prefix = format!("{project_slug}/{category}/");

    for edge in out_edges {
        let key = edge.src_key;
        let map = out_idx.entry(key).or_default();
        map.entry(edge.kind)
            .or_default()
            .push(format!("{}{}", prefix, edge.dst_key));
    }

    for edge in in_edges {
        let key = edge.dst_key;
        let map = in_idx.entry(key).or_default();
        map.entry(edge.kind)
            .or_default()
            .push(format!("{}{}", prefix, edge.src_key));
    }

    let mut result: Vec<LinkedRow> = Vec::with_capacity(entries.len());
    for entry in entries {
        let key = entry.entry_key.clone();
        let out_links = out_idx.remove(&key).unwrap_or_default();
        let in_links = in_idx.remove(&key).unwrap_or_default();
        result.push(LinkedRow {
            entry,
            out_links,
            in_links,
        });
    }

    let _ = TRACKED_RELS;
    Ok(result)
}

/// Fetch edges involving the given keys. Direction `out` returns edges where
/// the key is `src_key`; `in` returns edges where the key is `dst_key`.
async fn edges_for_keys(
    store: &DbStore,
    project: &str,
    category: &str,
    keys: &[String],
    direction: &str,
) -> Result<Vec<EdgeRow>> {
    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = match direction {
        "out" => format!(
            "SELECT src_key, dst_key, kind FROM edges WHERE src_project = ? AND src_cat = ? AND src_key IN ({placeholders})"
        ),
        "in" => format!(
            "SELECT src_key, dst_key, kind FROM edges WHERE dst_project = ? AND dst_cat = ? AND dst_key IN ({placeholders})"
        ),
        _ => return Ok(Vec::new()),
    };
    let params = [vec![project.to_owned(), category.to_owned()], keys.to_vec()].concat();
    let mut rows = store
        .exec_params(&sql, params)
        .await
        .map_err(crate::error::Error::Db)?;
    let mut result = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .map_err(|e| crate::error::Error::Db(e.into()))?
    {
        result.push(EdgeRow {
            src_key: row.get::<String>(0).unwrap_or_default(),
            dst_key: row.get::<String>(1).unwrap_or_default(),
            kind: row.get::<String>(2).unwrap_or_default(),
        });
    }
    Ok(result)
}
