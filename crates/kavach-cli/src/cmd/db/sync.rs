use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Sync current session state to `SurrealDB` as an event.
pub(super) fn run() -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    runtime.block_on(async {
        let session = kavach_session::get_or_create_session();
        let db = match kavach_surreal::open_default().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open SurrealDB: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(e) = kavach_surreal::apply_schema(&db).await {
            let msg = format!("error: schema apply: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let payload = serde_json::json!({
            "session_id": session.session_id,
            "turn_count": session.turn_count,
            "research_done": session.research_done,
            "files_modified": session.files_modified,
            "tasks_created": session.tasks_created,
            "tasks_completed": session.tasks_completed,
            "context_phase": session.context_phase,
        });
        let project_id = resolve_project_id(&db, &session.project, &session.work_dir).await;
        match kavach_surreal::append_event(
            &db,
            "session_sync",
            "kavach-cli",
            project_id,
            Some(&payload.to_string()),
        )
        .await
        {
            Ok(_) => {
                if let Err(io_err) = print_or_exit("synced session state to SurrealDB") {
                    return into_exit_code(io_err);
                }
                0
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
        }
    })
}

async fn resolve_project_id(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    slug: &str,
    workdir: &str,
) -> Option<surrealdb_types::RecordId> {
    if !slug.is_empty()
        && let Ok(Some(p)) = kavach_surreal::project_get_by_slug(db, slug).await
    {
        return p.id;
    }
    if !workdir.is_empty()
        && let Ok(Some(p)) = kavach_surreal::project_find_by_path(db, workdir).await
    {
        return p.id;
    }
    None
}
