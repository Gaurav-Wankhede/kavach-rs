// split: intentional - cohesive wipe_project module (single public fn + private helper)
// Wipe all data for a project — surgical delete across all project-scoped tables.
// Preserves other projects. Requires explicit --confirm flag for safety.
use crate::error::{Error, Result};
use crate::projects::get_by_slug;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
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

/// Allow-list of project-scoped tables a wipe may touch. A table NOT in this set
/// is rejected (fail-closed) — `verified_delete` interpolates the table name into
/// the query, so it must never accept an unvetted string.
fn is_wipeable_table(table: &str) -> bool {
    PROJECT_SCOPED_TABLES.contains(&table)
}

/// Count → DELETE → re-count-and-assert-zero for one project-scoped table.
///
/// This is the Layer-2 read-back-assertion law (decision.kavach-self-watchdog-design):
/// a self-DELETE must PROVE its effect, not assume it. The prior code counted
/// before deleting and returned that count as "deleted" — but a partial delete
/// (engine error mid-statement) would silently over-report. Here the post-count
/// is asserted zero; a non-zero residual is a hard `Error`, not a swallowed
/// success, so a partial wipe surfaces to the caller instead of reporting clean.
async fn verified_delete(db: &Surreal<Db>, table: &'static str, pid: &RecordId) -> Result<usize> {
    // Fail-closed: `table` is interpolated below, so reject anything off the
    // allow-list before it reaches the query string.
    if !is_wipeable_table(table) {
        return Err(Error::RecordNotFound(format!("unknown table: {table}")));
    }
    let before = query_count(db, table, pid).await?;

    let delete_q = format!("DELETE {table} WHERE project = $pid");
    db.query(&delete_q).bind(("pid", pid.clone())).await?;

    // Read-back proof: the table must be empty for this project now. A residual
    // means the DELETE only partially applied — fail loudly, never report clean.
    let after = query_count(db, table, pid).await?;
    if after != 0 {
        return Err(Error::RecordNotFound(format!(
            "wipe read-back assertion failed: {table} had {before} rows, {after} remain after DELETE (partial delete)"
        )));
    }
    Ok(before)
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
        let count = verified_delete(db, table, &pid).await?;
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
    // Same fail-closed allow-list as verified_delete: `table` is interpolated
    // into the count query, so reject anything unvetted.
    if is_wipeable_table(table) {
        query_count(db, table, pid).await
    } else {
        Err(Error::RecordNotFound(format!("unknown table: {table}")))
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

#[cfg(test)]
mod tests {
    use super::{PROJECT_SCOPED_TABLES, count_as_usize, is_wipeable_table};

    #[test]
    fn allow_list_accepts_every_scoped_table() {
        for &t in PROJECT_SCOPED_TABLES {
            assert!(is_wipeable_table(t), "{t} must be wipeable");
        }
    }

    #[test]
    fn allow_list_rejects_unvetted_or_injected_names() {
        // The name is interpolated into the DELETE/count query — anything off the
        // allow-list must be rejected before it can reach the query string.
        assert!(!is_wipeable_table("user"));
        assert!(!is_wipeable_table(""));
        assert!(!is_wipeable_table("decision WHERE 1=1; DELETE user"));
        assert!(!is_wipeable_table("DECISION"), "case-sensitive: no upper alias");
    }

    #[test]
    fn count_narrowing_is_saturating_not_panicking() {
        assert_eq!(count_as_usize(0), 0);
        assert_eq!(count_as_usize(42), 42);
        assert_eq!(count_as_usize(-1), 0, "negative count floors to 0, never panics");
    }
}
