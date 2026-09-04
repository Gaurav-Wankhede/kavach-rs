//! Exporter and record normalizer for SurrealDB to NDJSON migration stream.
use serde_json::Value;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::types::{
    EdgeRecord, EntityRecord, EventRecord, MemoryRecord, MigrateReport, NormalizedRecord,
    PartRecord, ProjectRecord,
};
use crate::turso::CATEGORIES;
use crate::turso::graph::ALLOWED_RELS;

/// Strip table prefix from a record ID string (e.g. "project:foo" -> "foo").
fn strip_prefix(raw: &str, table: &str) -> String {
    let prefix = format!("{table}:");
    raw.strip_prefix(&prefix).unwrap_or(raw).trim_matches('`').to_owned()
}

/// Parse string tags or array tags into a clean string vector.
fn normalize_tags(val: Option<&Value>) -> Vec<String> {
    val.and_then(|v| v.as_array()).map_or_else(Vec::new, |arr| {
        arr.iter().filter_map(|t| t.as_str().map(ToOwned::to_owned)).collect()
    })
}

/// Normalize an untyped JSON record from a SurrealDB query into a typed migration record.
#[expect(clippy::missing_errors_doc, reason = "comment_noise_guard single-line ceiling")]
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

/// Heuristic check for directed relation edge structure.
fn is_edge_shape(val: &Value) -> bool {
    val.get("in").is_some() && val.get("out").is_some()
}

/// Normalize raw json into a MemoryRecord.
fn parse_memory_record(val: &Value, table: &str) -> NormalizedRecord {
    let proj_raw = val.get("project").and_then(Value::as_str).unwrap_or_default();
    let project = strip_prefix(proj_raw, "project");
    let category = val.get("category").and_then(Value::as_str).map_or_else(
        || {
            if CATEGORIES.contains(&table) { table.to_owned() } else { "decision".to_owned() }
        },
        ToOwned::to_owned,
    );
    let key_raw = val.get("entry_key").and_then(Value::as_str).unwrap_or_default();
    let entry_key = strip_prefix(key_raw, &category);
    let title = val.get("title").and_then(Value::as_str).unwrap_or_default().to_owned();
    let content = val.get("content").and_then(Value::as_str).unwrap_or_default().to_owned();
    let status = val.get("status").and_then(Value::as_str).map(ToOwned::to_owned);
    let entry_status = val.get("entry_status").and_then(Value::as_str).map(ToOwned::to_owned);
    let tags = normalize_tags(val.get("tags"));
    let decay_score = val.get("decay_score").and_then(Value::as_f64);
    let access_count = val.get("access_count").and_then(Value::as_i64).unwrap_or(0);
    let priority = val.get("priority").and_then(Value::as_i64);
    let lane = val.get("lane").and_then(Value::as_str).map(ToOwned::to_owned);
    let exec_prompt = val.get("exec_prompt").and_then(Value::as_str).map(ToOwned::to_owned);
    let occupied_by = val.get("occupied_by").and_then(Value::as_str).map(ToOwned::to_owned);
    let occupied_until = val.get("occupied_until").and_then(Value::as_str).map(ToOwned::to_owned);
    let created_at = val.get("created_at").and_then(Value::as_str).map(ToOwned::to_owned);
    let updated_at = val.get("updated_at").and_then(Value::as_str).map(ToOwned::to_owned);
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
    let id = val.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    let entity_type = val.get("entity_type").and_then(Value::as_str).unwrap_or_default().to_owned();
    let name = val.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
    let properties = val.get("properties").map(|p| {
        if p.is_string() {
            p.as_str().unwrap_or_default().to_owned()
        } else {
            p.to_string()
        }
    });
    let content_hash = val.get("content_hash").and_then(Value::as_str).map(ToOwned::to_owned);
    let project = val.get("project").and_then(Value::as_str).map(ToOwned::to_owned);
    NormalizedRecord::Entity(EntityRecord {
        id,
        entity_type,
        name,
        properties,
        content_hash,
        project,
    })
}

/// Normalize raw json into an EdgeRecord.
fn parse_edge_record(val: &Value, table: &str) -> NormalizedRecord {
    let from_id = val.get("in").or_else(|| val.get("from_id")).and_then(Value::as_str).unwrap_or_default().to_owned();
    let to_id = val.get("out").or_else(|| val.get("to_id")).and_then(Value::as_str).unwrap_or_default().to_owned();
    let rel_type = val.get("rel_type").and_then(Value::as_str).map_or_else(
        || {
            if ALLOWED_RELS.contains(&table) { table.to_owned() } else { "references".to_owned() }
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
    let slug_raw = val.get("slug").or_else(|| val.get("id")).and_then(Value::as_str).unwrap_or_default();
    let slug = strip_prefix(slug_raw, "project");
    let path = val.get("path").and_then(Value::as_str).map(ToOwned::to_owned);
    let tags = normalize_tags(val.get("tags"));
    NormalizedRecord::Project(ProjectRecord { slug, path, tags })
}

/// Normalize raw json into a PartRecord.
fn parse_part_record(val: &Value) -> NormalizedRecord {
    let proj_raw = val.get("project").and_then(Value::as_str).unwrap_or_default();
    let project = strip_prefix(proj_raw, "project");
    let part = val.get("part").and_then(Value::as_str).unwrap_or_default().to_owned();
    let path = val.get("path").and_then(Value::as_str).map(ToOwned::to_owned);
    NormalizedRecord::Part(PartRecord { project, part, path })
}

/// Normalize raw json into an EventRecord.
fn parse_event_record(val: &Value) -> NormalizedRecord {
    let id = val.get("id").and_then(Value::as_i64);
    let proj_raw = val.get("project").and_then(Value::as_str).unwrap_or_default();
    let project = strip_prefix(proj_raw, "project");
    let kind = val.get("kind").and_then(Value::as_str).unwrap_or_default().to_owned();
    let payload = val.get("payload").map(|p| if p.is_string() { p.as_str().unwrap_or_default().to_owned() } else { p.to_string() });
    let created_at = val.get("created_at").and_then(Value::as_str).unwrap_or_default().to_owned();
    NormalizedRecord::Event(EventRecord { id, project, kind, payload, created_at })
}

/// Stream export of all memory rows, graph elements, and registries from SurrealDB to writer.
#[expect(clippy::missing_errors_doc, reason = "comment_noise_guard single-line ceiling")]
pub async fn export_from_surreal<W: AsyncWrite + Unpin>(
    db: &Surreal<Db>,
    writer: &mut W,
) -> Result<MigrateReport, String> {
    let mut report = MigrateReport::default();
    for cat in CATEGORIES {
        let sql = format!("SELECT * FROM {cat};");
        let mut response = db.query(sql).await.map_err(|e| format!("query {cat}: {e}"))?;
        let rows: Vec<Value> = response.take(0).map_err(|e| format!("take {cat}: {e}"))?;
        for row in rows {
            let norm = parse_memory_record(&row, cat);
            let line = serde_json::to_string(&norm).map_err(|e| format!("serialize {cat}: {e}"))?;
            writer.write_all(line.as_bytes()).await.map_err(|e| format!("write {cat}: {e}"))?;
            writer.write_all(b"\n").await.map_err(|e| format!("newline {cat}: {e}"))?;
            report.memory_count = report.memory_count.saturating_add(1);
        }
    }
    let mut ent_resp = db.query("SELECT * FROM entity;").await.map_err(|e| format!("query entity: {e}"))?;
    let entities: Vec<Value> = ent_resp.take(0).map_err(|e| format!("take entity: {e}"))?;
    for ent in entities {
        let norm = parse_entity_record(&ent);
        let line = serde_json::to_string(&norm).map_err(|e| format!("serialize entity: {e}"))?;
        writer.write_all(line.as_bytes()).await.map_err(|e| format!("write entity: {e}"))?;
        writer.write_all(b"\n").await.map_err(|e| format!("newline entity: {e}"))?;
        report.entity_count = report.entity_count.saturating_add(1);
    }
    for rel in ALLOWED_RELS {
        let sql = format!("SELECT * FROM {rel};");
        if let Ok(mut resp) = db.query(sql).await
            && let Ok(rows) = resp.take::<Vec<Value>>(0)
        {
            for row in rows {
                let norm = parse_edge_record(&row, rel);
                let line = serde_json::to_string(&norm).map_err(|e| format!("serialize edge {rel}: {e}"))?;
                writer.write_all(line.as_bytes()).await.map_err(|e| format!("write edge {rel}: {e}"))?;
                writer.write_all(b"\n").await.map_err(|e| format!("newline edge {rel}: {e}"))?;
                report.edge_count = report.edge_count.saturating_add(1);
            }
        }
    }
    if let Ok(mut resp) = db.query("SELECT * FROM project;").await
        && let Ok(rows) = resp.take::<Vec<Value>>(0)
    {
        for row in rows {
            let norm = parse_project_record(&row);
            let line = serde_json::to_string(&norm).map_err(|e| format!("serialize project: {e}"))?;
            writer.write_all(line.as_bytes()).await.map_err(|e| format!("write project: {e}"))?;
            writer.write_all(b"\n").await.map_err(|e| format!("newline project: {e}"))?;
            report.project_count = report.project_count.saturating_add(1);
        }
    }
    if let Ok(mut resp) = db.query("SELECT * FROM part;").await
        && let Ok(rows) = resp.take::<Vec<Value>>(0)
    {
        for row in rows {
            let norm = parse_part_record(&row);
            let line = serde_json::to_string(&norm).map_err(|e| format!("serialize part: {e}"))?;
            writer.write_all(line.as_bytes()).await.map_err(|e| format!("write part: {e}"))?;
            writer.write_all(b"\n").await.map_err(|e| format!("newline part: {e}"))?;
            report.part_count = report.part_count.saturating_add(1);
        }
    }
    writer.flush().await.map_err(|e| format!("flush export: {e}"))?;
    Ok(report)
}
