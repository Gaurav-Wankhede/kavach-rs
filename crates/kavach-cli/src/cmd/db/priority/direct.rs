// Direct-DB fallback for `db priority-set` when daemon is unavailable.
// Mirrors the resilient-open + project-resolve pattern of status_update::run.

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};
use kavach_types::Priority;

pub(super) fn run_direct(
    project_slug: &str,
    category: &str,
    key: &str,
    effective: Option<i64>,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            return super::write_err(&format!("error: tokio runtime: {e}"));
        }
    };
    runtime.block_on(async {
        let db = match crate::cmd::db::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => {
                return super::write_err(&format!("error: open SurrealDB: {e}"));
            }
        };
        if let Err(e) = kavach_surreal::apply_schema(&db).await {
            return super::write_err(&format!("error: schema apply: {e}"));
        }
        let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return super::write_err(&format!("error: project not found: {project_slug}"));
            }
            Err(e) => {
                return super::write_err(&format!("error: {e}"));
            }
        };
        let Some(project_id) = project.id else {
            if let Err(io_err) = ewrite_or_exit("error: project has no id") {
                return into_exit_code(io_err);
            }
            return 1;
        };
        let priority = effective.map(Priority::new);
        match kavach_surreal::set_priority(&db, category, &project_id, key, priority).await {
            Ok(_id) => super::print_pretty(category, key, effective, false),
            Err(e) => super::write_err(&format!("error: {e}")),
        }
    })
}
