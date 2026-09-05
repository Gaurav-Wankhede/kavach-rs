use crate::error::Result;
use crate::turso::TursoStore;
use kavach_types::Priority;

/// Surgical priority mutation — partial UPDATE of `priority` + `updated_at` only.
///
/// Title, content, status, `entry_status` are untouched. Use for human-in-loop
/// reranking without re-supplying full row data. Returns the row id on success,
/// or `Err(RecordNotFound)` if no row matches the (project, key) pair.
///
/// `new_priority = Some(n)` sets the priority; `None` clears it back to NULL
/// (FIFO tail in the dispatch sort).
///
/// # Errors
/// `Error::RecordNotFound` when no row matches (project, key); `Error::Turso`
/// when the UPDATE itself fails.
pub async fn set_priority(
    db: &TursoStore,
    category: &str,
    project_id: &str,
    entry_key: &str,
    new_priority: Option<Priority>,
) -> Result<String> {
    if category != "decision" && category != "roadmap" {
        return Err(crate::error::Error::RecordNotFound(format!(
            "priority is only defined on roadmap and decision tables, got: {category}"
        )));
    }
    let now = crate::turso::now_rfc3339();
    let mut response = db
        .exec_params(
            "UPDATE memories SET priority = $priority, updated_at = $now \
             WHERE project = $pid AND category = $cat AND entry_key = $key \
             RETURNING entry_key",
            libsql::params![
                new_priority.map(Priority::get),
                now,
                project_id.to_owned(),
                category.to_owned(),
                entry_key.to_owned()
            ],
        )
        .await?;
    match response.next().await? {
        Some(r) => Ok(r.get::<String>(0)?),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "{category}/{entry_key} not found in project"
        ))),
    }
}

/// Surgical lane mutation — partial UPDATE of `lane` + `updated_at` only.
///
/// Lane is the dispatch-affinity slice a session runs (`KAVACH_LANE`). Roadmap
/// only. Title/content/status/priority are untouched. `Some(name)` pins the
/// card to that lane; `None` clears it back to the unlaned general backlog.
///
/// # Errors
/// `Error::RecordNotFound` when no row matches (project, key) or the category is
/// not `roadmap`; `Error::Turso` when the UPDATE itself fails.
pub async fn set_lane(
    db: &TursoStore,
    category: &str,
    project_id: &str,
    entry_key: &str,
    new_lane: Option<String>,
) -> Result<String> {
    if category != "roadmap" {
        return Err(crate::error::Error::RecordNotFound(format!(
            "lane is only defined on the roadmap table, got: {category}"
        )));
    }
    let now = crate::turso::now_rfc3339();
    let mut response = db
        .exec_params(
            "UPDATE memories SET lane = $lane, updated_at = $now \
             WHERE project = $pid AND category = $cat AND entry_key = $key \
             RETURNING entry_key",
            libsql::params![
                new_lane,
                now,
                project_id.to_owned(),
                category.to_owned(),
                entry_key.to_owned()
            ],
        )
        .await?;
    match response.next().await? {
        Some(r) => Ok(r.get::<String>(0)?),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "roadmap/{entry_key} not found in project"
        ))),
    }
}
