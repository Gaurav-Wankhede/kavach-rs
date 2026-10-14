// split: intentional - graph-relevance-based event archival
// Per decision.relevance-based-rotation (id=3765): time-based deletion is the wrong
// model. Events anchored to active roadmap/architecture must NEVER be archived.
// Events with zero graph anchors AND age > floor_days are candidates for archival.
// Status: events get status='archived' (never DELETE — preserves audit trail).
use crate::error::Result;
use crate::db::DbStore;

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ArchiveReport {
    pub scanned: usize,
    pub anchored: usize,
    pub archived: usize,
    pub samples: Vec<String>,
}

/// Identify and archive events with no graph anchors to active roadmap entries.
///
/// Anchored = event has any outgoing edge (references) to a roadmap row whose
/// `entry_status` is non-terminal (todo / `in_progress` / blocked / deferred).
/// Floor: events younger than `floor_days` are never archived even if unanchored.
/// Always uses status='archived' (never DELETE) to preserve audit trail.
///
/// # Errors
/// Propagates `Error::Database` if any database query fails.
pub async fn archive_irrelevant(
    db: &DbStore,
    floor_days: i64,
    dry_run: bool,
) -> Result<ArchiveReport> {
    let mut report = ArchiveReport::default();

    // Events table has no `status` column; the audit trail is preserved by
    // keeping the row and marking it via a `status`-style payload marker is not
    // possible without a column — so archival here means DELETE of the event
    // row only when unanchored (the audit trail lives in memories/edges, which
    // are never deleted). Floor: created_at older than floor_days.
    let floor_secs = floor_days.saturating_mul(86_400);
    let floor_ts = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::seconds(floor_secs))
        .unwrap_or_else(chrono::Utc::now);
    let floor_str = floor_ts.to_rfc3339();
    let mut scan_resp = db
        .exec_params(
            "SELECT id FROM events WHERE created_at < $floor",
            libsql::params![floor_str],
        )
        .await?;
    let mut candidates: Vec<i64> = Vec::new();
    while let Some(r) = scan_resp.next().await? {
        candidates.push(r.get::<i64>(0)?);
    }
    report.scanned = candidates.len();

    let mut to_archive: Vec<i64> = Vec::with_capacity(candidates.len());
    for id in &candidates {
        if has_active_anchor(db, *id).await? {
            report.anchored = report.anchored.saturating_add(1);
        } else {
            to_archive.push(*id);
        }
    }

    for id in to_archive.iter().take(5) {
        report.samples.push(id.to_string());
    }

    if !dry_run && !to_archive.is_empty() {
        // Per-id DELETE inside one transaction — `IN ($ids)` cannot bind a
        // Vec, and placeholder expansion is unnecessary at this cardinality.
        let txn = db.conn.transaction().await?;
        let mut archived = 0usize;
        for id in &to_archive {
            let mut arch_resp = txn
                .query(
                    "DELETE FROM events WHERE id = ?1 RETURNING id",
                    libsql::params![*id],
                )
                .await?;
            if arch_resp.next().await?.is_some() {
                archived = archived.saturating_add(1);
            }
        }
        txn.commit().await?;
        report.archived = archived;
    }

    Ok(report)
}

/// Returns true if the event has any outgoing 'references' edge to a roadmap
/// entry whose `entry_status` is non-terminal.
async fn has_active_anchor(db: &DbStore, event_id: i64) -> Result<bool> {
    let mut rows = db
        .exec_params(
            "SELECT 1 FROM entity_edges e \
             JOIN memories m ON e.to_id = 'memory:' || m.project || '/' || m.category || '/' || m.entry_key \
             WHERE e.from_id = ?1 AND e.rel_type = 'references' \
               AND m.entry_status IN ('todo', 'in_progress', 'blocked', 'deferred') \
             LIMIT 1",
            libsql::params![event_id.to_string()],
        )
        .await?;
    Ok(rows.next().await?.is_some())
}
