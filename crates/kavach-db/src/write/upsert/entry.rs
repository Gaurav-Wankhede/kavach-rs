// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use crate::error::Result;
use crate::turso::TursoStore;
use kavach_types::Priority;

/// Idempotent upsert on (project, category, `entry_key`).
///
/// # Errors
/// Unknown category or query failure.
pub async fn upsert_entry(
    db: &TursoStore,
    category: &str,
    project_id: &str,
    entry_key: &str,
    title: &str,
    content: &str,
    priority: Option<Priority>,
) -> Result<String> {
    match category {
        "decision" | "research" | "roadmap" | "pattern" | "app_spec" => {}
        other => {
            return Err(crate::error::Error::Migration(format!(
                "unknown category: {other}"
            )));
        }
    }
    let now = crate::turso::now_rfc3339();
    let priority_i64 = priority.map(Priority::get);
    let mut response = db
        .exec_params(
            "INSERT INTO memories (project, category, entry_key, title, content, status, priority, created_at, updated_at) \
             VALUES ($project, $cat, $key, $title, $content, 'active', $priority, $now, $now) \
             ON CONFLICT(project, category, entry_key) DO UPDATE SET \
                 title = excluded.title, content = excluded.content, \
                 status = 'active', \
                 priority = COALESCE(excluded.priority, memories.priority), \
                 updated_at = excluded.updated_at \
             RETURNING entry_key",
            libsql::params![
                project_id.to_owned(),
                category.to_owned(),
                entry_key.to_owned(),
                title.to_owned(),
                content.to_owned(),
                priority_i64,
                now
            ],
        )
        .await?;
    match response.next().await? {
        Some(r) => Ok(r.get::<String>(0)?),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "upsert returned empty for {category}/{entry_key}"
        ))),
    }
}
