// split: intentional - cohesive wipe_project module (single public fn + private helper)
// Wipe all data for a project — surgical delete across all project-scoped tables.
// Preserves other projects. Requires explicit --confirm flag for safety.
use crate::error::{Error, Result};
use crate::projects::get_by_slug;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

/// Result of wiping a project.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct WipeReport {
    pub project_slug: String,
    pub tables: Vec<(&'static str, usize)>,
    pub project_deleted: bool,
}

#[derive(surrealdb_types::SurrealValue)]
struct CountRow {
    count: i64,
}

/// Explicit i64 -> usize narrowing for `SurrealDB` `COUNT()` rows.
/// Replaces `c as usize` (silent truncation on 32-bit targets); `COUNT()` is
/// non-negative, so `try_from` only fails when count exceeds `usize::MAX` (would
/// require >2^32 rows on 32-bit). Returns 0 on underflow/overflow rather than
/// panicking — wipe is best-effort and the actual delete already executed.
fn count_as_usize(c: i64) -> usize {
    usize::try_from(c).unwrap_or(0)
}

async fn delete_table(db: &Surreal<Db>, table: &'static str, pid: &RecordId) -> Result<usize> {
    // Count first, then delete - avoids deserialization issues with RETURN BEFORE
    let count: usize = match table {
        "decision" => {
            let mut resp = db
                .query("SELECT count() FROM decision WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE decision WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "research" => {
            let mut resp = db
                .query("SELECT count() FROM research WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE research WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "pattern" => {
            let mut resp = db
                .query("SELECT count() FROM pattern WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE pattern WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "roadmap" => {
            let mut resp = db
                .query("SELECT count() FROM roadmap WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE roadmap WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "app_spec" => {
            let mut resp = db
                .query("SELECT count() FROM app_spec WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE app_spec WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "event" => {
            let mut resp = db
                .query("SELECT count() FROM event WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE event WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        other => return delete_extra_table(db, other, pid).await,
    };
    Ok(count)
}

async fn delete_extra_table(
    db: &Surreal<Db>,
    table: &'static str,
    pid: &RecordId,
) -> Result<usize> {
    let count: usize = match table {
        "part" => {
            let mut resp = db
                .query("SELECT count() FROM part WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE part WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "entity" => {
            let mut resp = db
                .query("SELECT count() FROM entity WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE entity WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "session" => {
            let mut resp = db
                .query("SELECT count() FROM session WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE session WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "arch_decision" => {
            let mut resp = db
                .query("SELECT count() FROM arch_decision WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE arch_decision WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "algo_decision" => {
            let mut resp = db
                .query("SELECT count() FROM algo_decision WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE algo_decision WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        "gate_pattern" => {
            let mut resp = db
                .query("SELECT count() FROM gate_pattern WHERE project = $pid GROUP ALL")
                .bind(("pid", pid.clone()))
                .await?;
            let row: Option<CountRow> = resp.take(0)?;
            let c = row.map_or(0, |cr| cr.count);
            db.query("DELETE gate_pattern WHERE project = $pid")
                .bind(("pid", pid.clone()))
                .await?;
            count_as_usize(c)
        }
        other => return Err(Error::RecordNotFound(format!("unknown table: {other}"))),
    };
    Ok(count)
}

const PROJECT_SCOPED_TABLES: &[&str] = &[
    "decision",
    "research",
    "pattern",
    "roadmap",
    "app_spec",
    "event",
    "part",
    "entity",
    "session",
    "arch_decision",
    "algo_decision",
    "gate_pattern",
];

/// Wipe all data for a project. Returns counts per table.
/// Does NOT delete `rag_tree` rows (they are global, not project-scoped).
///
/// # Errors
/// Returns `Error::RecordNotFound` when `slug` does not match any project or
/// the matched project lacks an id. Propagates `Error::Surreal` from the
/// underlying queries.
pub async fn wipe_project(db: &Surreal<Db>, slug: &str) -> Result<WipeReport> {
    let project = get_by_slug(db, slug).await?;
    let project = project.ok_or_else(|| Error::RecordNotFound(format!("project '{slug}'")))?;
    let pid: RecordId = project
        .id
        .ok_or_else(|| Error::RecordNotFound(format!("project '{slug}' has no id")))?;

    let mut report = WipeReport {
        project_slug: slug.to_owned(),
        tables: Vec::with_capacity(PROJECT_SCOPED_TABLES.len()),
        project_deleted: false,
    };

    for &table in PROJECT_SCOPED_TABLES {
        let count = delete_table(db, table, &pid).await?;
        report.tables.push((table, count));
    }

    // Delete the project registry row itself
    db.query("DELETE $pid").bind(("pid", pid.clone())).await?;
    report.project_deleted = true;

    Ok(report)
}

async fn query_count(db: &Surreal<Db>, table: &'static str, pid: &RecordId) -> Result<usize> {
    let query_str = format!("SELECT count() FROM {table} WHERE project = $pid GROUP ALL");
    let mut resp = db.query(&query_str).bind(("pid", pid.clone())).await?;
    let row: Option<CountRow> = resp.take(0)?;
    Ok(count_as_usize(row.map_or(0, |cr| cr.count)))
}

async fn count_table(db: &Surreal<Db>, table: &'static str, pid: &RecordId) -> Result<usize> {
    match table {
        "decision" | "research" | "pattern" | "roadmap" | "app_spec" | "event" | "part"
        | "entity" | "session" | "arch_decision" | "algo_decision" | "gate_pattern" => {
            query_count(db, table, pid).await
        }
        other => Err(Error::RecordNotFound(format!("unknown table: {other}"))),
    }
}

/// Preview what would be wiped without deleting anything.
///
/// # Errors
/// Returns `Error::RecordNotFound` for an unknown slug or missing project id.
/// Propagates `Error::Surreal` from the underlying count queries.
pub async fn preview_wipe(db: &Surreal<Db>, slug: &str) -> Result<WipeReport> {
    let project = get_by_slug(db, slug).await?;
    let project = project.ok_or_else(|| Error::RecordNotFound(format!("project '{slug}'")))?;
    let pid: RecordId = project
        .id
        .ok_or_else(|| Error::RecordNotFound(format!("project '{slug}' has no id")))?;

    let mut report = WipeReport {
        project_slug: slug.to_owned(),
        tables: Vec::with_capacity(PROJECT_SCOPED_TABLES.len()),
        project_deleted: false,
    };

    for &table in PROJECT_SCOPED_TABLES {
        let count = count_table(db, table, &pid).await?;
        report.tables.push((table, count));
    }

    Ok(report)
}
