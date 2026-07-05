// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use super::super::status::UpdatedIdRow;
use crate::error::Result;
use kavach_types::Priority;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

/// Idempotent upsert on (project, `entry_key`).
///
/// # Errors
/// Unknown category or query failure.
pub async fn upsert_entry(
    db: &Surreal<Db>,
    category: &str,
    project_id: &RecordId,
    entry_key: &str,
    title: &str,
    content: &str,
    priority: Option<Priority>,
) -> Result<RecordId> {
    let pk = crate::key_str::project_key_str(project_id);
    let rid = RecordId::new(category, format!("{pk}:{entry_key}"));
    let query = match category {
        "decision" => {
            "LET $eid = (SELECT VALUE id FROM decision WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'decision', entry_key = $key, title = $title, content = $content, status = 'active', priority = IF $priority != NONE THEN $priority ELSE priority END, updated_at = time::now() RETURN id"
        }
        "research" => {
            "LET $eid = (SELECT VALUE id FROM research WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'research', entry_key = $key, title = $title, content = $content, status = 'active', updated_at = time::now() RETURN id"
        }
        "roadmap" => {
            "LET $eid = (SELECT VALUE id FROM roadmap WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'roadmap', entry_key = $key, title = $title, content = $content, status = 'active', priority = IF $priority != NONE THEN $priority ELSE priority END, updated_at = time::now() RETURN id"
        }
        "pattern" => {
            "LET $eid = (SELECT VALUE id FROM pattern WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'pattern', entry_key = $key, title = $title, content = $content, status = 'active', updated_at = time::now() RETURN id"
        }
        "app_spec" => {
            "LET $eid = (SELECT VALUE id FROM app_spec WHERE project = $project AND entry_key = $key LIMIT 1)[0] ?? $rid; UPSERT $eid SET project = $project, category = 'app_spec', entry_key = $key, title = $title, content = $content, status = 'active', updated_at = time::now() RETURN id"
        }
        other => {
            return Err(crate::error::Error::Migration(format!(
                "unknown category: {other}"
            )));
        }
    };

    let priority_i64 = priority.map(Priority::get);
    let mut response = db
        .query(query)
        .bind(("rid", rid))
        .bind(("project", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .bind(("title", title.to_owned()))
        .bind(("content", content.to_owned()))
        .bind(("priority", priority_i64))
        .await?;
    let rows: Vec<UpdatedIdRow> = response.take(1)?;
    match rows.into_iter().next() {
        Some(r) => Ok(r.id),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "upsert returned empty for {category}/{entry_key}"
        ))),
    }
}
