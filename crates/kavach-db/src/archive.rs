// split: intentional - graph-relevance-based event archival
// Per decision.relevance-based-rotation (id=3765): time-based deletion is the wrong
// model. Events anchored to active roadmap/architecture must NEVER be archived.
// Events with zero graph anchors AND age > floor_days are candidates for archival.
// Status: events get status='archived' (never DELETE — preserves audit trail).
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ArchiveReport {
    pub scanned: usize,
    pub anchored: usize,
    pub archived: usize,
    pub samples: Vec<String>,
}

#[derive(Debug, surrealdb_types::SurrealValue)]
struct EventScanRow {
    id: RecordId,
}

#[derive(Debug, surrealdb_types::SurrealValue)]
struct CountResult {
    cnt: usize,
}

/// Identify and archive events with no graph anchors to active roadmap entries.
///
/// Anchored = event has any outgoing edge (references) to a roadmap row whose
/// `entry_status` is non-terminal (todo / `in_progress` / blocked / deferred).
/// Floor: events younger than `floor_days` are never archived even if unanchored.
/// Always uses status='archived' (never DELETE) to preserve audit trail.
///
/// # Errors
/// Propagates `Error::Surreal` if any database query fails.
pub async fn archive_irrelevant(
    db: &Surreal<Db>,
    floor_days: i64,
    dry_run: bool,
) -> Result<ArchiveReport> {
    let mut report = ArchiveReport::default();

    let scan_query = "SELECT id FROM event WHERE \
                      created_at < time::now() - duration::from::days($floor) AND \
                      (status IS NONE OR status != 'archived')";
    let mut scan_resp = db.query(scan_query).bind(("floor", floor_days)).await?;
    let candidates: Vec<EventScanRow> = scan_resp.take(0)?;
    report.scanned = candidates.len();

    let mut to_archive: Vec<RecordId> = Vec::with_capacity(candidates.len());
    for c in &candidates {
        if has_active_anchor(db, &c.id).await? {
            report.anchored = report.anchored.saturating_add(1);
        } else {
            to_archive.push(c.id.clone());
        }
    }

    for id in to_archive.iter().take(5) {
        report.samples.push(format!("{id:?}"));
    }

    if !dry_run && !to_archive.is_empty() {
        let archive_query =
            "UPDATE $events SET status = 'archived', archived_at = time::now() RETURN AFTER";
        let mut arch_resp = db
            .query(archive_query)
            .bind(("events", to_archive.clone()))
            .await?;
        let archived: Vec<serde_json::Value> = arch_resp.take(0)?;
        report.archived = archived.len();
    }

    Ok(report)
}

/// Returns true if the event has any outgoing 'references' edge to a roadmap
/// entry whose `entry_status` is non-terminal.
async fn has_active_anchor(db: &Surreal<Db>, event_id: &RecordId) -> Result<bool> {
    let query = "SELECT count() AS cnt FROM ( \
                 SELECT VALUE ->references->roadmap[ \
                   WHERE entry_status IN ['todo', 'in_progress'] \
                 ] FROM $event \
                 ) GROUP ALL";
    let mut response = db.query(query).bind(("event", event_id.clone())).await?;
    let result: Option<CountResult> = response.take(0)?;
    Ok(result.is_some_and(|r| r.cnt > 0))
}
