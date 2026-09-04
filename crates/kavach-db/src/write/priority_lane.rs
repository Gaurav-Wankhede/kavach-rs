use crate::error::Result;
use kavach_types::Priority;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

/// Surgical priority mutation — partial UPDATE of `priority` + `updated_at` only.
///
/// Title, content, status, `entry_status` are untouched. Use for human-in-loop
/// reranking without re-supplying full row data. Returns the row id on success,
/// or `Err(RecordNotFound)` if no row matches the (project, key) pair.
///
/// `new_priority = Some(n)` sets the priority; `None` clears it back to NONE
/// (FIFO tail in the dispatch sort).
///
/// # Errors
/// `Error::RecordNotFound` when no row matches (project, key); `Error::Surreal`
/// when the UPDATE itself fails or returns malformed shape.
pub async fn set_priority(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    new_priority: Option<Priority>,
) -> Result<RecordId> {
    let query = match category {
        "decision" => {
            "UPDATE decision SET priority = $priority, updated_at = time::now() WHERE project = $pid AND entry_key = $key RETURN id"
        }
        "roadmap" => {
            "UPDATE roadmap SET priority = $priority, updated_at = time::now() WHERE project = $pid AND entry_key = $key RETURN id"
        }
        _ => {
            return Err(crate::error::Error::RecordNotFound(format!(
                "priority is only defined on roadmap and decision tables, got: {category}"
            )));
        }
    };
    let mut response = db
        .query(query)
        .bind(("pid", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("priority", new_priority.map(Priority::get)))
        .await?;
    let ids: Vec<RecordId> = response.take("id")?;
    ids.into_iter().next().ok_or_else(|| {
        crate::error::Error::RecordNotFound(format!("{category}/{entry_key} not found in project"))
    })
}

/// Surgical lane mutation — partial UPDATE of `lane` + `updated_at` only.
///
/// Lane is the dispatch-affinity slice a session runs (`KAVACH_LANE`). Roadmap
/// only. Title/content/status/priority are untouched. `Some(name)` pins the
/// card to that lane; `None` clears it back to the unlaned general backlog.
///
/// # Errors
/// `Error::RecordNotFound` when no row matches (project, key) or the category is
/// not `roadmap`; `Error::Surreal` when the UPDATE itself fails.
pub async fn set_lane(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    new_lane: Option<String>,
) -> Result<RecordId> {
    if category != "roadmap" {
        return Err(crate::error::Error::RecordNotFound(format!(
            "lane is only defined on the roadmap table, got: {category}"
        )));
    }
    let mut response = db
        .query(
            "UPDATE roadmap SET lane = $lane, updated_at = time::now() \
             WHERE project = $pid AND entry_key = $key RETURN id",
        )
        .bind(("pid", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("lane", new_lane))
        .await?;
    let ids: Vec<RecordId> = response.take("id")?;
    ids.into_iter().next().ok_or_else(|| {
        crate::error::Error::RecordNotFound(format!("roadmap/{entry_key} not found in project"))
    })
}
