// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use crate::error::Result;
use crate::turso::TursoStore;
use kavach_types::Priority;

use super::graph_stmts::append_entity_graph;

/// Atomic upsert: entry + event + graph in one txn.
#[bon::builder(finish_fn = build_for_call)]
pub async fn upsert_entry_full(
    db: &TursoStore,
    category: &str,
    project_id: &str,
    entry_key: &str,
    title: &str,
    content: &str,
    event_source: &str,
    qualified_name: &str,
    references: &[String],
    priority: Option<Priority>,
    exec_prompt: Option<&str>,
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
    let project_name = crate::key_str::project_key_str(project_id);
    let rid_returned = format!("{project_name}:{entry_key}");
    let _ = event_source; // events table has no `source` column; kind carries the origin.

    crate::retry::with_retry(|| async {
        // K-1 root cause: fresh roadmap rows used to store `entry_status =
        // NULL` (only `status = 'active'` was written), so every board had to
        // guess the lane. Seed the lane at creation for roadmap rows; other
        // categories leave it NULL (their lifecycle lives in `status`).
        // Computed inside the retry closure: it is consumed by the params
        // below and the closure may run more than once.
        let initial_lane: Option<String> = (category == "roadmap").then(|| "todo".to_owned());
        let txn = db.conn.transaction().await?;
        // `Connection::query` defers statement errors into the returned Rows
        // (they surface only on `next()`); these are INSERTs, so use `execute`,
        // which returns the error immediately (§no_silent_failure).
        txn.execute(
            "INSERT INTO memories (project, category, entry_key, title, content, status, entry_status, priority, exec_prompt, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?8, ?9, ?9) \
             ON CONFLICT(project, category, entry_key) DO UPDATE SET \
                 title = excluded.title, content = excluded.content, \
                 status = 'active', \
                 entry_status = COALESCE(memories.entry_status, excluded.entry_status), \
                 priority = COALESCE(excluded.priority, memories.priority), \
                 exec_prompt = COALESCE(excluded.exec_prompt, memories.exec_prompt), \
                 updated_at = excluded.updated_at",
            libsql::params![
                project_id.to_owned(),
                category.to_owned(),
                entry_key.to_owned(),
                title.to_owned(),
                content.to_owned(),
                initial_lane,
                priority_i64,
                exec_prompt.map(str::to_owned),
                now.clone()
            ],
        )
        .await?;
        // Keep FTS in sync (parity with `EntryStore::upsert_entry`): the
        // production write path must populate the search index, or `kavach db
        // write` rows never surface in FTS results.
        txn.execute(
            "DELETE FROM memories_fts WHERE project=?1 AND category=?2 AND entry_key=?3",
            libsql::params![
                project_id.to_owned(),
                category.to_owned(),
                entry_key.to_owned()
            ],
        )
        .await?;
        txn.execute(
            "INSERT INTO memories_fts (entry_key, project, category, title, content) \
             VALUES (?1,?2,?3,?4,?5)",
            libsql::params![
                entry_key.to_owned(),
                project_id.to_owned(),
                category.to_owned(),
                title.to_owned(),
                content.to_owned()
            ],
        )
        .await?;
        txn.execute(
            "INSERT INTO events (project, kind, payload, created_at) \
             VALUES (?1, 'memory_write', ?2, ?3)",
            libsql::params![
                project_id.to_owned(),
                format!("{{\"category\":\"{category}\",\"entry_key\":\"{entry_key}\"}}"),
                now.clone()
            ],
        )
        .await?;
        append_entity_graph(&txn, qualified_name, &project_name, references).await?;
        txn.commit().await?;
        Ok(rid_returned.clone())
    })
    .await
}
