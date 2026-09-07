// split: intentional - cohesive wipe_project module (single public fn + private helper)
// Wipe all data for a project — surgical delete across all project-scoped tables.
// Preserves other projects. Requires explicit --confirm flag for safety.
use crate::error::{Error, Result};
use crate::turso::TursoStore;
use crate::turso::entries::EntryStore;

/// Result of wiping a project.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct WipeReport {
    pub project_slug: String,
    pub tables: Vec<(&'static str, usize)>,
    pub project_deleted: bool,
}

/// Allow-list of project-scoped tables a wipe may touch. A table NOT in this set
/// is rejected (fail-closed) — verified_delete interpolates the table name into
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
async fn verified_delete(
    db: &TursoStore,
    table: &'static str,
    project_slug: &str,
) -> Result<usize> {
    if !is_wipeable_table(table) {
        return Err(Error::RecordNotFound(format!("unknown table: {table}")));
    }
    let entries = db
        .list_by_category(project_slug, table)
        .await
        .map_err(Error::Turso)?;
    let before = entries.len();
    for entry in entries {
        db.delete_by_key(project_slug, table, &entry.entry_key)
            .await
            .map_err(Error::Turso)?;
    }
    let after = db
        .list_by_category(project_slug, table)
        .await
        .map_err(Error::Turso)?
        .len();
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
    "gate_config",
];

/// Wipe all data for a project. Returns counts per table.
/// Does NOT delete `rag_tree` rows (they are global, not project-scoped).
///
/// # Errors
/// Returns `Error::RecordNotFound` when `slug` does not match any project.
/// Propagates `Error::Turso` from the underlying queries.
pub async fn wipe_project(db: &TursoStore, slug: &str) -> Result<WipeReport> {
    let project = crate::projects::get_by_slug(db, slug).await?;
    let project = project.ok_or_else(|| Error::RecordNotFound(format!("project '{slug}'")))?;
    let project_slug = project.slug;

    let mut report = WipeReport {
        project_slug: slug.to_owned(),
        tables: Vec::with_capacity(PROJECT_SCOPED_TABLES.len()),
        project_deleted: false,
    };

    for &table in PROJECT_SCOPED_TABLES {
        let count = verified_delete(db, table, &project_slug).await?;
        report.tables.push((table, count));
    }

    db.exec("DELETE FROM projects WHERE slug = ?")
        .await
        .map_err(Error::Turso)?;
    report.project_deleted = true;

    Ok(report)
}

/// Preview what would be wiped without deleting anything.
///
/// # Errors
/// Returns `Error::RecordNotFound` for an unknown slug or missing project id.
/// Propagates `Error::Turso` from the underlying count queries.
pub async fn preview_wipe(db: &TursoStore, slug: &str) -> Result<WipeReport> {
    let project = crate::projects::get_by_slug(db, slug).await?;
    let project = project.ok_or_else(|| Error::RecordNotFound(format!("project '{slug}'")))?;
    let project_slug = project.slug;

    let mut report = WipeReport {
        project_slug: slug.to_owned(),
        tables: Vec::with_capacity(PROJECT_SCOPED_TABLES.len()),
        project_deleted: false,
    };

    for &table in PROJECT_SCOPED_TABLES {
        let entries = db
            .list_by_category(&project_slug, table)
            .await
            .map_err(Error::Turso)?;
        report.tables.push((table, entries.len()));
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{PROJECT_SCOPED_TABLES, is_wipeable_table};

    #[test]
    fn allow_list_accepts_every_scoped_table() {
        for &t in PROJECT_SCOPED_TABLES {
            assert!(is_wipeable_table(t), "{t} must be wipeable");
        }
    }

    #[test]
    fn allow_list_rejects_unvetted_or_injected_names() {
        assert!(!is_wipeable_table("user"));
        assert!(!is_wipeable_table(""));
        assert!(!is_wipeable_table("decision WHERE 1=1; DELETE user"));
        assert!(
            !is_wipeable_table("DECISION"),
            "case-sensitive: no upper alias"
        );
    }
}
