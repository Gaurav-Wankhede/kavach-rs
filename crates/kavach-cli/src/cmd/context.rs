// Unified context payload for AI agent harness — single JSON with kanban, session, phase, loop state.
// SOURCE: https://dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no
// PATTERN: Single structured JSON payload per GitLab glab CLI pattern.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(
    project: &str,
    limit: usize,
    status_filter: Option<&str>,
    key_filter: Option<&str>,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!(r#"{{"error":"tokio: {e}"}}"#);
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async { run_async(project, limit, status_filter, key_filter).await })
}

async fn run_async(
    project_slug: &str,
    limit: usize,
    status_filter: Option<&str>,
    key_filter: Option<&str>,
) -> i32 {
    // Session state
    let session = kavach_session::get_or_create_session();

    // DB connection
    let db = match kavach_surreal::open_default().await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!(r#"{{"error":"db: {e}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // Project lookup
    let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let msg = format!(r#"{{"error":"project not found: {project_slug}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!(r#"{{"error":"{e}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    let Some(project_id) = project.id else {
        if let Err(io_err) = print_or_exit(r#"{"error":"project has no id"}"#) {
            return into_exit_code(io_err);
        }
        return 1;
    };

    // Kanban items
    let roadmap = match kavach_surreal::list_by_project(&db, "roadmap", &project_id).await {
        Ok(rows) => rows,
        Err(e) => {
            let msg = format!(r#"{{"error":"{e}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // 3-state workflow band: ready → in-flight → completed-pending-verify.
    let open_statuses = ["todo", "in_progress", "done"];
    let mut open: Vec<_> = roadmap
        .iter()
        .filter(|e| open_statuses.contains(&e.entry_status_str()))
        .filter(|e| status_filter.is_none_or(|s| e.entry_status_str() == s))
        .filter(|e| key_filter.is_none_or(|k| e.entry_key.contains(k)))
        .collect();

    // Sort: in_progress first
    open.sort_by_key(|e| i32::from(e.entry_status_str() != "in_progress"));

    let total = open.len();
    let displayed: Vec<_> = if limit == 0 {
        open.into_iter().cloned().collect()
    } else {
        open.into_iter().take(limit).cloned().collect()
    };

    build_and_print_payload(project_slug, &session, &roadmap, &displayed, limit, total)
}

fn build_and_print_payload(
    project_slug: &str,
    session: &kavach_session::SessionState,
    roadmap: &[kavach_surreal::MemoryEntry],
    displayed: &[kavach_surreal::MemoryEntry],
    limit: usize,
    total: usize,
) -> i32 {
    // Build kanban JSON items
    let items_json: Vec<String> = displayed
        .iter()
        .map(|e| {
            format!(
                r#"{{"key":"{}","status":"{}","title":"{}"}}"#,
                e.entry_key.replace('"', r#"\""#),
                e.entry_status_str(),
                e.title.replace('"', r#"\""#)
            )
        })
        .collect();

    let in_progress_count = roadmap
        .iter()
        .filter(|e| e.entry_status_str() == "in_progress")
        .count();
    let todo_count = roadmap
        .iter()
        .filter(|e| e.entry_status_str() == "todo")
        .count();
    let done_count = roadmap
        .iter()
        .filter(|e| e.entry_status_str() == "done")
        .count();
    let verified_count = roadmap
        .iter()
        .filter(|e| e.entry_status_str() == "verified")
        .count();

    let payload = format!(
        r#"{{"project":"{project_slug}","session":{{"id":"{sid}","turn":{turn},"phase":"{phase}"}},"loop":{{"active":{active},"target":"{target}","iteration":{iter},"max":{max}}},"kanban":{{"in_progress":{ip},"todo":{td},"done":{dn},"verified":{vf}}},"items":[{items}],"total":{total},"displayed":{disp},"has_more":{more}}}"#,
        sid = session.session_id,
        turn = session.turn_count,
        phase = session.current_phase,
        active = session.loop_active,
        target = session.loop_target,
        iter = session.loop_iteration,
        max = session.loop_max_iterations,
        ip = in_progress_count,
        td = todo_count,
        dn = done_count,
        vf = verified_count,
        items = items_json.join(","),
        disp = displayed.len(),
        more = limit > 0 && total > limit,
    );

    if let Err(io_err) = print_or_exit(&payload) {
        return into_exit_code(io_err);
    }
    0
}
