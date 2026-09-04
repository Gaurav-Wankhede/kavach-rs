//! Streaming batch importer from NDJSON records into TursoStore.
use std::io::BufRead;

use chrono::{DateTime, Utc};

use super::types::{ImportOpts, MigrateReport, NormalizedRecord};
use crate::dual_write::MemoryEntry;
use crate::turso::TursoStore;
use crate::turso::entries::EntryStore;
use crate::turso::graph::{GraphEntity, GraphStore};

/// Parse an RFC3339 timestamp string into UTC DateTime.
fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(DateTime::<Utc>::from)
}

/// Execute import of NDJSON records into TursoStore with batch transactions.
#[expect(clippy::missing_errors_doc, reason = "comment_noise_guard single-line ceiling")]
pub async fn import_records(
    store: &TursoStore,
    records: &[NormalizedRecord],
    opts: ImportOpts,
) -> Result<MigrateReport, String> {
    let mut report = MigrateReport::default();
    if opts.dry_run {
        for record in records {
            count_record(record, &mut report);
        }
        return Ok(report);
    }
    if !opts.overwrite {
        let existing = store.list_all().await.map_err(|e| format!("check existing: {e}"))?;
        if !existing.is_empty() {
            return Err("target database already contains memories (use --overwrite to proceed)".to_owned());
        }
    }
    import_pass_one(store, records, opts.batch_size, &mut report).await?;
    import_pass_two(store, records, opts.batch_size, &mut report).await?;
    Ok(report)
}

/// Stream-read NDJSON from a reader and collect typed records with line diagnostics.
#[expect(clippy::missing_errors_doc, reason = "comment_noise_guard single-line ceiling")]
pub fn read_ndjson<R: BufRead>(reader: R) -> Result<Vec<NormalizedRecord>, String> {
    let mut out = Vec::new();
    for (idx, line_res) in reader.lines().enumerate() {
        let raw = line_res.map_err(|e| format!("read line {idx}: {e}"))?;
        if raw.trim().is_empty() {
            continue;
        }
        let line_no = idx.saturating_add(1);
        let val: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("parse json at line {line_no}: {e}"))?;
        let record = super::export::normalize_record(&val)
            .map_err(|e| format!("normalize record at line {line_no}: {e}"))?;
        out.push(record);
    }
    Ok(out)
}

/// Increment appropriate counter on the migration report.
const fn count_record(record: &NormalizedRecord, report: &mut MigrateReport) {
    match record {
        NormalizedRecord::Memory(_) => report.memory_count = report.memory_count.saturating_add(1),
        NormalizedRecord::Entity(_) => report.entity_count = report.entity_count.saturating_add(1),
        NormalizedRecord::Edge(_) => report.edge_count = report.edge_count.saturating_add(1),
        NormalizedRecord::Project(_) => report.project_count = report.project_count.saturating_add(1),
        NormalizedRecord::Part(_) => report.part_count = report.part_count.saturating_add(1),
        NormalizedRecord::Event(_) => report.event_count = report.event_count.saturating_add(1),
    }
}

/// Pass 1 inserts memories, entities, projects, parts, and events.
async fn import_pass_one(
    store: &TursoStore,
    records: &[NormalizedRecord],
    batch_size: usize,
    report: &mut MigrateReport,
) -> Result<(), String> {
    let mut in_batch = 0usize;
    store.exec("BEGIN TRANSACTION").await.map_err(|e| format!("begin tx: {e}"))?;
    for record in records {
        match record {
            NormalizedRecord::Memory(m) => {
                let entry = MemoryEntry {
                    id: None,
                    project: crate::turso::fake_record_id(&m.project),
                    category: Some(m.category.clone()),
                    entry_key: m.entry_key.clone(),
                    title: m.title.clone(),
                    content: m.content.clone(),
                    status: m.status.clone(),
                    entry_status: m.entry_status.clone(),
                    tags: Some(m.tags.clone()),
                    decay_score: m.decay_score,
                    access_count: Some(m.access_count),
                    created_at: m.created_at.as_deref().and_then(parse_ts),
                    updated_at: m.updated_at.as_deref().and_then(parse_ts),
                    priority: m.priority,
                    lane: m.lane.clone(),
                    exec_prompt: m.exec_prompt.clone(),
                    occupied_by: m.occupied_by.clone(),
                    occupied_until: m.occupied_until.as_deref().and_then(parse_ts),
                };
                store.upsert_entry(&entry).await.map_err(|e| format!("upsert entry: {e}"))?;
                report.memory_count = report.memory_count.saturating_add(1);
                in_batch = in_batch.saturating_add(1);
            }
            NormalizedRecord::Entity(e) => {
                let ge = GraphEntity::new(
                    e.id.clone(),
                    e.entity_type.clone(),
                    e.name.clone(),
                    e.properties.clone(),
                    e.content_hash.clone(),
                    e.project.clone(),
                );
                store.upsert_entity(&ge).await.map_err(|err| format!("upsert entity: {err}"))?;
                report.entity_count = report.entity_count.saturating_add(1);
                in_batch = in_batch.saturating_add(1);
            }
            NormalizedRecord::Project(p) => {
                let tags_json = serde_json::to_string(&p.tags).unwrap_or_else(|_| "[]".to_owned());
                let sql = "INSERT INTO projects (slug, path, tags) VALUES (?1, ?2, ?3) \
                           ON CONFLICT(slug) DO UPDATE SET path=excluded.path, tags=excluded.tags";
                store.conn.execute(sql, libsql::params![p.slug.clone(), p.path.clone(), tags_json])
                    .await
                    .map_err(|e| format!("upsert project: {e}"))?;
                report.project_count = report.project_count.saturating_add(1);
                in_batch = in_batch.saturating_add(1);
            }
            NormalizedRecord::Part(pt) => {
                let sql = "INSERT INTO parts (project, part, path) VALUES (?1, ?2, ?3) \
                           ON CONFLICT(project, part) DO UPDATE SET path=excluded.path";
                store.conn.execute(sql, libsql::params![pt.project.clone(), pt.part.clone(), pt.path.clone()])
                    .await
                    .map_err(|e| format!("upsert part: {e}"))?;
                report.part_count = report.part_count.saturating_add(1);
                in_batch = in_batch.saturating_add(1);
            }
            NormalizedRecord::Event(ev) => {
                let payload_str = ev.payload.as_deref().unwrap_or("{}");
                store.append_event(&ev.project, &ev.kind, payload_str).await.map_err(|e| format!("append event: {e}"))?;
                report.event_count = report.event_count.saturating_add(1);
                in_batch = in_batch.saturating_add(1);
            }
            NormalizedRecord::Edge(_) => {}
        }
        if in_batch >= batch_size {
            store.exec("COMMIT").await.map_err(|e| format!("commit tx: {e}"))?;
            store.exec("BEGIN TRANSACTION").await.map_err(|e| format!("begin tx: {e}"))?;
            in_batch = 0;
        }
    }
    store.exec("COMMIT").await.map_err(|e| format!("final commit pass 1: {e}"))?;
    Ok(())
}

/// Pass 2 relates edges now that all endpoints are guaranteed to exist.
async fn import_pass_two(
    store: &TursoStore,
    records: &[NormalizedRecord],
    batch_size: usize,
    report: &mut MigrateReport,
) -> Result<(), String> {
    let mut in_batch = 0usize;
    store.exec("BEGIN TRANSACTION").await.map_err(|e| format!("begin tx: {e}"))?;
    for record in records {
        if let NormalizedRecord::Edge(ed) = record {
            if let Err(e) = store.relate_dynamic(&ed.from_id, &ed.to_id, &ed.rel_type, ed.weight).await {
                report.warnings.push(format!("skip edge {} -{}-> {}: {e}", ed.from_id, ed.rel_type, ed.to_id));
            } else {
                report.edge_count = report.edge_count.saturating_add(1);
                in_batch = in_batch.saturating_add(1);
            }
        }
        if in_batch >= batch_size {
            store.exec("COMMIT").await.map_err(|e| format!("commit tx: {e}"))?;
            store.exec("BEGIN TRANSACTION").await.map_err(|e| format!("begin tx: {e}"))?;
            in_batch = 0;
        }
    }
    store.exec("COMMIT").await.map_err(|e| format!("final commit pass 2: {e}"))?;
    Ok(())
}
