//! Exporter and record normalizer for TursoStore to NDJSON migration stream.
use serde_json::Value;
use std::collections::HashSet;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::types::{
    EdgeRecord, EntityRecord, EventRecord, MemoryRecord, MigrateReport, NormalizedRecord,
    PartRecord, ProjectRecord,
};
use crate::turso::CATEGORIES;
use crate::turso::TursoStore;
use crate::turso::graph::ALLOWED_RELS;

/// Strip table prefix from a record ID string (e.g. "project:foo" -> "foo").
fn strip_prefix(raw: &str, table: &str) -> String {
    let prefix = format!("{table}:");
    raw.strip_prefix(&prefix)
        .unwrap_or(raw)
        .trim_matches('`')
        .to_owned()
}

/// Parse string tags or array tags into a clean string vector.
fn normalize_tags(val: Option<&Value>) -> Vec<String> {
    val.and_then(|v| v.as_array()).map_or_else(Vec::new, |arr| {
        arr.iter()
            .filter_map(|t| t.as_str().map(ToOwned::to_owned))
            .collect()
    })
}

/// Normalize an untyped JSON record from a TursoStore query into a typed migration record.
#[expect(
    clippy::missing_errors_doc,
    reason = "comment_noise_guard single-line ceiling"
)]
pub fn normalize_record(raw: &Value) -> Result<NormalizedRecord, String> {
    if raw.get("kind").is_some() {
        return serde_json::from_value(raw.clone())
            .map_err(|e| format!("decode typed record: {e}"));
    }
    let table = raw.get("table").and_then(Value::as_str).unwrap_or_default();
    let record = raw.get("record").unwrap_or(raw);
    if CATEGORIES.contains(&table) || is_memory_shape(record) {
        return Ok(parse_memory_record(record, table));
    }
    if table == "entity" || is_entity_shape(record) {
        return Ok(parse_entity_record(record));
    }
    if table == "citation" || is_citation_shape(record) {
        return Ok(parse_citation_record(record));
    }
    if ALLOWED_RELS.contains(&table) || is_edge_shape(record) {
        return Ok(parse_edge_record(record, table));
    }
    if table == "project" {
        return Ok(parse_project_record(record));
    }
    if table == "part" {
        return Ok(parse_part_record(record));
    }
    if table == "event" {
        return Ok(parse_event_record(record));
    }
    Err(format!("unrecognized record structure for table '{table}'"))
}

/// Heuristic check for memory row structure.
fn is_memory_shape(val: &Value) -> bool {
    val.get("entry_key").is_some() && val.get("title").is_some()
}

/// Heuristic check for knowledge graph entity structure.
fn is_entity_shape(val: &Value) -> bool {
    val.get("entity_type").is_some() && val.get("name").is_some() && val.get("id").is_some()
}

/// Heuristic check for citation row structure.
fn is_citation_shape(val: &Value) -> bool {
    val.get("metadata").is_some() && val.get("name").is_some() && val.get("entry_key").is_some()
}

/// Heuristic check for directed relation edge structure.
fn is_edge_shape(val: &Value) -> bool {
    val.get("in").is_some() && val.get("out").is_some()
}

/// Normalize an endpoint ID string, stripping TursoStore Debug wrappers and backticks.
#[must_use]
pub fn normalize_endpoint(raw: &str) -> String {
    let s = raw.trim().trim_matches('`');
    if let Some((_, rest)) = s.split_once("Table(\"")
        && let Some((table, after_table)) = rest.split_once('"')
        && let Some((_, after_key)) = after_table.split_once("key:")
        && let Some((_, after_string)) = after_key.split_once("String(\"")
        && let Some((inner_key, _)) = after_string.rsplit_once('"')
    {
        let clean_key = inner_key
            .replace("\\\"", "\"")
            .replace("String(\"", "")
            .replace("\")", "")
            .replace('`', "");
        return format!("{table}:{clean_key}");
    }
    let cleaned = s.replace('`', "");
    if let Some((cat, rest)) = cleaned.split_once(":String(\"")
        && let Some((proj, key)) = rest.split_once("\"):")
    {
        return format!("{cat}:{proj}:{key}");
    }
    cleaned
}

/// Normalize raw json into a MemoryRecord.
fn parse_memory_record(val: &Value, table: &str) -> NormalizedRecord {
    let proj_raw = val
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let project = strip_prefix(proj_raw, "project");
    let category = val.get("category").and_then(Value::as_str).map_or_else(
        || {
            if CATEGORIES.contains(&table) {
                table.to_owned()
            } else {
                "decision".to_owned()
            }
        },
        ToOwned::to_owned,
    );
    let key_raw = val
        .get("entry_key")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let entry_key = strip_prefix(key_raw, &category);
    let title = val
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let content = val
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let status = val
        .get("status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let entry_status = val
        .get("entry_status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let tags = normalize_tags(val.get("tags"));
    let decay_score = val.get("decay_score").and_then(Value::as_f64);
    let access_count = val.get("access_count").and_then(Value::as_i64).unwrap_or(0);
    let priority = val.get("priority").and_then(Value::as_i64);
    let lane = val
        .get("lane")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let exec_prompt = val
        .get("exec_prompt")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let occupied_by = val
        .get("occupied_by")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let occupied_until = val
        .get("occupied_until")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let created_at = val
        .get("created_at")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let updated_at = val
        .get("updated_at")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    NormalizedRecord::Memory(Box::new(MemoryRecord {
        project,
        category,
        entry_key,
        title,
        content,
        status,
        entry_status,
        tags,
        decay_score,
        access_count,
        priority,
        lane,
        exec_prompt,
        occupied_by,
        occupied_until,
        created_at,
        updated_at,
    }))
}

/// Normalize raw json into an EntityRecord.
fn parse_entity_record(val: &Value) -> NormalizedRecord {
    let id_raw = val.get("id").and_then(Value::as_str).unwrap_or_default();
    let id = normalize_endpoint(id_raw);
    let entity_type = val
        .get("entity_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let name = val
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let properties = val.get("properties").map(|p| {
        if p.is_string() {
            p.as_str().unwrap_or_default().to_owned()
        } else {
            p.to_string()
        }
    });
    let content_hash = val
        .get("content_hash")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let project = val
        .get("project")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    NormalizedRecord::Entity(EntityRecord {
        id,
        entity_type,
        name,
        properties,
        content_hash,
        project,
    })
}

/// Normalize raw json citation row into an EntityRecord.
fn parse_citation_record(val: &Value) -> NormalizedRecord {
    let id_raw = val.get("id").and_then(Value::as_str).unwrap_or_default();
    let id = normalize_endpoint(id_raw);
    let entity_type = "citation".to_owned();
    let name = val
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let properties = val.get("metadata").map(|p| {
        if p.is_string() {
            p.as_str().unwrap_or_default().to_owned()
        } else {
            p.to_string()
        }
    });
    let proj_raw = val
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let project = if proj_raw.is_empty() {
        None
    } else {
        Some(strip_prefix(proj_raw, "project"))
    };
    NormalizedRecord::Entity(EntityRecord {
        id,
        entity_type,
        name,
        properties,
        content_hash: None,
        project,
    })
}

/// Normalize raw json into an EdgeRecord.
fn parse_edge_record(val: &Value, table: &str) -> NormalizedRecord {
    let from_raw = val
        .get("in")
        .or_else(|| val.get("from_id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let from_id = normalize_endpoint(from_raw);
    let to_raw = val
        .get("out")
        .or_else(|| val.get("to_id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let to_id = normalize_endpoint(to_raw);
    let rel_type = val.get("rel_type").and_then(Value::as_str).map_or_else(
        || {
            if ALLOWED_RELS.contains(&table) {
                table.to_owned()
            } else {
                "references".to_owned()
            }
        },
        ToOwned::to_owned,
    );
    let weight = val.get("weight").and_then(Value::as_f64).unwrap_or(1.0);
    NormalizedRecord::Edge(EdgeRecord {
        from_id,
        rel_type,
        to_id,
        weight,
    })
}

/// Normalize raw json into a ProjectRecord.
fn parse_project_record(val: &Value) -> NormalizedRecord {
    let slug_raw = val
        .get("slug")
        .or_else(|| val.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let slug = strip_prefix(slug_raw, "project");
    let path = val
        .get("path")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let tags = normalize_tags(val.get("tags"));
    NormalizedRecord::Project(ProjectRecord { slug, path, tags })
}

/// Normalize raw json into a PartRecord.
fn parse_part_record(val: &Value) -> NormalizedRecord {
    let proj_raw = val
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let project = strip_prefix(proj_raw, "project");
    let part = val
        .get("part")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let path = val
        .get("path")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    NormalizedRecord::Part(PartRecord {
        project,
        part,
        path,
    })
}

/// Normalize raw json into an EventRecord.
fn parse_event_record(val: &Value) -> NormalizedRecord {
    let id = val.get("id").and_then(Value::as_i64);
    let proj_raw = val
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let project = strip_prefix(proj_raw, "project");
    let kind = val
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let payload = val.get("payload").map(|p| {
        if p.is_string() {
            p.as_str().unwrap_or_default().to_owned()
        } else {
            p.to_string()
        }
    });
    let created_at = val
        .get("created_at")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    NormalizedRecord::Event(EventRecord {
        id,
        project,
        kind,
        payload,
        created_at,
    })
}

/// Convert a libsql row into a JSON object keyed by column name (string values
/// for TEXT, numbers for INTEGER/REAL, null otherwise). The parse_* functions
/// read by key, so column names must match the record shapes.
fn row_to_value(row: &libsql::Row) -> Result<Value, String> {
    let mut obj = serde_json::Map::new();
    let ncols = row.column_count();
    for i in 0..ncols {
        let col = row.column_name(i).unwrap_or_default().to_owned();
        let v = match row.get::<Option<String>>(i) {
            Ok(Some(s)) => Value::String(s),
            Ok(None) => Value::Null,
            Err(_) => match row.get::<Option<i64>>(i) {
                Ok(Some(n)) => Value::Number(n.into()),
                Ok(None) => Value::Null,
                Err(_) => match row.get::<Option<f64>>(i) {
                    Ok(Some(f)) => serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                    Ok(None) => Value::Null,
                    Err(_) => Value::Null,
                },
            },
        };
        obj.insert(col, v);
    }
    Ok(Value::Object(obj))
}

/// Map a `citations` row to the shape `parse_citation_record` expects
/// (`id`, `name`, `metadata`, `project`).
fn citation_row_to_value(row: &libsql::Row) -> Result<Value, String> {
    let project = row
        .get::<String>(0)
        .map_err(|e| format!("citation project: {e}"))?;
    let entry_key = row
        .get::<String>(1)
        .map_err(|e| format!("citation entry_key: {e}"))?;
    let name = row
        .get::<String>(2)
        .map_err(|e| format!("citation name: {e}"))?;
    let slug = row
        .get::<String>(3)
        .map_err(|e| format!("citation slug: {e}"))?;
    let url = row
        .get::<String>(4)
        .map_err(|e| format!("citation url: {e}"))?;
    let created_at = row
        .get::<String>(5)
        .map_err(|e| format!("citation created_at: {e}"))?;
    let updated_at = row
        .get::<String>(6)
        .map_err(|e| format!("citation updated_at: {e}"))?;
    let metadata = serde_json::json!({
        "slug": slug,
        "url": url,
        "created_at": created_at,
        "updated_at": updated_at,
    });
    Ok(serde_json::json!({
        "id": format!("citation:{project}/{entry_key}"),
        "name": name,
        "metadata": metadata,
        "project": project,
    }))
}

/// Map a `parts` row to the shape `parse_part_record` expects (`project`, `part`, `path`).
fn part_row_to_value(row: &libsql::Row) -> Result<Value, String> {
    let project = row
        .get::<String>(0)
        .map_err(|e| format!("part project: {e}"))?;
    let part = row
        .get::<String>(1)
        .map_err(|e| format!("part name: {e}"))?;
    let path = row
        .get::<String>(2)
        .map_err(|e| format!("part path: {e}"))?;
    Ok(serde_json::json!({ "project": project, "part": part, "path": path }))
}

/// Stream export of all memory rows, graph elements, and registries from TursoStore to writer.
#[expect(
    clippy::missing_errors_doc,
    reason = "comment_noise_guard single-line ceiling"
)]
pub async fn export_from_surreal<W: AsyncWrite + Unpin>(
    db: &TursoStore,
    writer: &mut W,
) -> Result<MigrateReport, String> {
    let mut report = MigrateReport::default();
    for cat in CATEGORIES {
        let mut rows = db
            .exec_params(
                "SELECT * FROM memories WHERE category = ?1",
                libsql::params![cat.to_owned()],
            )
            .await
            .map_err(|e| format!("query memories {cat}: {e}"))?;
        while let Some(row) = rows.next().await.map_err(|e| format!("next {cat}: {e}"))? {
            let val = row_to_value(&row)?;
            let norm = parse_memory_record(&val, cat);
            let line = serde_json::to_string(&norm).map_err(|e| format!("serialize {cat}: {e}"))?;
            writer
                .write_all(line.as_bytes())
                .await
                .map_err(|e| format!("write {cat}: {e}"))?;
            writer
                .write_all(b"\n")
                .await
                .map_err(|e| format!("newline {cat}: {e}"))?;
            report.memory_count = report.memory_count.saturating_add(1);
        }
    }
    let mut ent_resp = db
        .exec_params("SELECT * FROM entities", ())
        .await
        .map_err(|e| format!("query entities: {e}"))?;
    while let Some(row) = ent_resp
        .next()
        .await
        .map_err(|e| format!("next entity: {e}"))?
    {
        let val = row_to_value(&row)?;
        let norm = parse_entity_record(&val);
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize entity: {e}"))?;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write entity: {e}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("newline entity: {e}"))?;
        report.entity_count = report.entity_count.saturating_add(1);
    }
    let mut cit_resp = db
        .exec_params("SELECT * FROM citations", ())
        .await
        .map_err(|e| format!("query citations: {e}"))?;
    while let Some(row) = cit_resp
        .next()
        .await
        .map_err(|e| format!("next citation: {e}"))?
    {
        let val = citation_row_to_value(&row)?;
        let norm = parse_citation_record(&val);
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize citation: {e}"))?;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write citation: {e}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("newline citation: {e}"))?;
        report.entity_count = report.entity_count.saturating_add(1);
    }
    let mut seen_edges = HashSet::new();
    let mut edge_resp = db
        .exec_params("SELECT * FROM entity_edges", ())
        .await
        .map_err(|e| format!("query entity_edges: {e}"))?;
    while let Some(row) = edge_resp
        .next()
        .await
        .map_err(|e| format!("next edge: {e}"))?
    {
        let val = row_to_value(&row)?;
        let norm = parse_edge_record(&val, "");
        if let NormalizedRecord::Edge(ref ed) = norm {
            let key = (ed.from_id.clone(), ed.rel_type.clone(), ed.to_id.clone());
            if !seen_edges.insert(key) {
                continue;
            }
        }
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize edge: {e}"))?;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write edge: {e}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("newline edge: {e}"))?;
        report.edge_count = report.edge_count.saturating_add(1);
    }
    let mut proj_resp = db
        .exec_params("SELECT * FROM projects", ())
        .await
        .map_err(|e| format!("query projects: {e}"))?;
    while let Some(row) = proj_resp
        .next()
        .await
        .map_err(|e| format!("next project: {e}"))?
    {
        let val = row_to_value(&row)?;
        let norm = parse_project_record(&val);
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize project: {e}"))?;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write project: {e}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("newline project: {e}"))?;
        report.project_count = report.project_count.saturating_add(1);
    }
    let mut part_resp = db
        .exec_params("SELECT * FROM parts", ())
        .await
        .map_err(|e| format!("query parts: {e}"))?;
    while let Some(row) = part_resp
        .next()
        .await
        .map_err(|e| format!("next part: {e}"))?
    {
        let val = part_row_to_value(&row)?;
        let norm = parse_part_record(&val);
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize part: {e}"))?;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write part: {e}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("newline part: {e}"))?;
        report.part_count = report.part_count.saturating_add(1);
    }
    let mut ev_resp = db
        .exec_params("SELECT * FROM events", ())
        .await
        .map_err(|e| format!("query events: {e}"))?;
    while let Some(row) = ev_resp
        .next()
        .await
        .map_err(|e| format!("next event: {e}"))?
    {
        let val = row_to_value(&row)?;
        let norm = parse_event_record(&val);
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize event: {e}"))?;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("write event: {e}"))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| format!("newline event: {e}"))?;
        report.event_count = report.event_count.saturating_add(1);
    }
    writer
        .flush()
        .await
        .map_err(|e| format!("flush export: {e}"))?;
    Ok(report)
}
