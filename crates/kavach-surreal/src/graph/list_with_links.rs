// One-round-trip fetch of category rows + their typed graph neighbours.
//
// [RCA]
// symptom:    The Roadmap / Kanban / Decisions tabs render flat category rows
//             with zero awareness of cross-category links in the graph.
// repro:      crates/kavach-app/src/pages/{roadmap,kanban,decisions}.rs:35
//             all end with kavach_surreal::list_by_project(...) — flat SELECT,
//             no graph traversal.
// why5:       Dashboard treats one SurrealDB as if it were 4 isolated DBs.
//             The SurrealDB value-prop (multi-model joins in one query) is
//             unused at the read path.
// root_cause: No row-with-neighbourhood fetch fn over per-category tables.
// fix:        This module. ONE SurrealQL block: SELECT rows from <category>,
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
// SOURCE: https://surrealdb.com/docs/learn/data-models/graph/overview
use crate::dual_write::MemoryEntry;
use crate::error::Result;
use serde::Deserialize;
use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

/// A row plus its inbound + outbound graph neighbours grouped by relation type.
#[derive(Debug, Clone, SurrealValue)]
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

#[derive(Deserialize, surrealdb_types::SurrealValue, Default)]
struct GraphRow {
    name: String,
    #[serde(default)]
    out_depends_on: Vec<String>,
    #[serde(default)]
    out_blocks: Vec<String>,
    #[serde(default)]
    out_supersedes: Vec<String>,
    #[serde(default)]
    out_references: Vec<String>,
    #[serde(default)]
    out_mentions: Vec<String>,
    #[serde(default)]
    in_depends_on: Vec<String>,
    #[serde(default)]
    in_blocks: Vec<String>,
    #[serde(default)]
    in_supersedes: Vec<String>,
    #[serde(default)]
    in_references: Vec<String>,
    #[serde(default)]
    in_mentions: Vec<String>,
}

/// Fetch category rows with their inbound and outbound graph neighbours.
///
/// # Errors
/// Propagates `Error::Surreal` when the `SurrealDB` query fails.
pub async fn list_with_links(
    db: &Surreal<Db>,
    category: &str,
    project_slug: &str,
) -> Result<Vec<LinkedRow>> {
    #[derive(surrealdb_types::SurrealValue)]
    struct IdRow {
        id: RecordId,
    }

    let proj_q = "SELECT id FROM project WHERE slug = $slug LIMIT 1";
    let mut proj_resp = db
        .query(proj_q)
        .bind(("slug", project_slug.to_owned()))
        .await?;
    let row: Option<IdRow> = proj_resp.take(0)?;
    let pid = match row {
        Some(r) => r.id,
        None => return Ok(Vec::new()),
    };

    let rows_q = "SELECT id, project, entry_key, title, content, status, entry_status, \
                  access_count, created_at, updated_at \
                  FROM type::table($table) WHERE project = $pid ORDER BY entry_key";
    let mut rows_resp = db
        .query(rows_q)
        .bind(("table", category.to_owned()))
        .bind(("pid", pid.clone()))
        .await?;
    let entries: Vec<MemoryEntry> = rows_resp.take(0)?;

    let prefix = format!("{project_slug}/{category}/");
    let graph_q = "SELECT name, \
            ->depends_on->entity.name  AS out_depends_on, \
            ->blocks->entity.name      AS out_blocks, \
            ->supersedes->entity.name  AS out_supersedes, \
            ->references->entity.name  AS out_references, \
            ->mentions->entity.name    AS out_mentions, \
            <-depends_on<-entity.name  AS in_depends_on, \
            <-blocks<-entity.name      AS in_blocks, \
            <-supersedes<-entity.name  AS in_supersedes, \
            <-references<-entity.name  AS in_references, \
            <-mentions<-entity.name    AS in_mentions \
         FROM entity WHERE entity_type = 'memory' AND string::starts_with(name, $prefix);";
    let mut graph_resp = db.query(graph_q).bind(("prefix", prefix.clone())).await?;
    let graph_rows: Vec<GraphRow> = graph_resp.take(0)?;

    let prefix_ref = prefix.as_str();
    let mut out_idx: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    let mut in_idx: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    for gr in graph_rows {
        let key = match gr.name.strip_prefix(prefix_ref) {
            Some(k) => k.to_owned(),
            None => continue,
        };
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        out.insert("depends_on".into(), gr.out_depends_on);
        out.insert("blocks".into(), gr.out_blocks);
        out.insert("supersedes".into(), gr.out_supersedes);
        out.insert("references".into(), gr.out_references);
        out.insert("mentions".into(), gr.out_mentions);
        let mut inb: HashMap<String, Vec<String>> = HashMap::new();
        inb.insert("depends_on".into(), gr.in_depends_on);
        inb.insert("blocks".into(), gr.in_blocks);
        inb.insert("supersedes".into(), gr.in_supersedes);
        inb.insert("references".into(), gr.in_references);
        inb.insert("mentions".into(), gr.in_mentions);
        out_idx.insert(key.clone(), out);
        in_idx.insert(key, inb);
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
