// split: intentional - cohesive pipeline subcommand (plan + status share the project resolver)
// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
// `kavach pipeline` — initializer→subagent pipeline (planner only).
// Database-backed: app_spec → roadmap items via kavach_db::upsert_entry_full
// (atomic memory entry + event + entity + graph edges in one transaction).
use crate::cli::PipelineAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn run(action: PipelineAction) -> i32 {
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
        let db = match kavach_db::open_default().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open Database: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        match action {
            PipelineAction::Plan { project, spec_key } => plan_async(&db, &project, &spec_key).await,
            PipelineAction::Status { project } => status_async(&db, &project).await,
        }
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "plan_async: linear pipeline initialization flow — project resolution + spec loading + task iteration + upsert loop"
)]
async fn plan_async(db: &kavach_db::DbStore, project_slug: &str, spec_key: &str) -> i32 {
    let project = match kavach_db::project_get_by_slug(db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let msg = format!("error: project not found: {project_slug}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if let Err(code) = crate::cmd::db::validate_project_workdir(&project) {
        return code;
    }
    let Some(project_id) = project.id else {
        if let Err(io_err) = ewrite_or_exit("error: project has no id") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let spec = match kavach_db::get_by_key(&db, "app_spec", &project_id, spec_key).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            let msg = format!("error: app_spec '{spec_key}' not found");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let json: serde_json::Value = match serde_json::from_str(&spec.content) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error: app_spec content is not valid JSON: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let Some(tasks) = json.get("tasks").and_then(|v| v.as_array()) else {
        if let Err(io_err) = ewrite_or_exit("error: app_spec must contain a 'tasks' array") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let head = format!(
        "[PIPELINE plan] reading {} tasks from app_spec '{}'",
        tasks.len(),
        spec_key
    );
    if let Err(io_err) = print_or_exit(&head) {
        return into_exit_code(io_err);
    }
    let mut batch: Vec<(String, String, String, String, Vec<String>)> = Vec::new();
    let mut skipped: usize = 0;

    for (idx, task) in tasks.iter().enumerate() {
        let task_key = task
            .get("key")
            .and_then(|v| v.as_str())
            .map_or_else(|| format!("{spec_key}-task-{idx}"), ToOwned::to_owned);
        let Some(task_title) = task.get("title").and_then(|v| v.as_str()) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        let task_content = match task.get("content") {
            None => "",
            Some(v) => {
                if let Some(s) = v.as_str() {
                    s
                } else {
                    skipped = skipped.saturating_add(1);
                    continue;
                }
            }
        };
        let qualified_name =
            kavach_engine::memory_entry_qualified_name("roadmap", &task_key, project_slug);
        let refs: Vec<String> = kavach_engine::extract_memory_entry_references(task_content);
        batch.push((task_key, task_title.to_owned(), task_content.to_owned(), qualified_name, refs));
    }

    if !batch.is_empty() {
        match kavach_db::upsert_entries_batch(&db, &project_id, "roadmap", &batch).await {
            Ok(written) => {
                for (task_key, task_title) in &written {
                    let line = format!("  → roadmap[todo] {task_key} — {task_title}");
                    if let Err(io_err) = print_or_exit(&line) {
                        return into_exit_code(io_err);
                    }
                }
            }
            Err(e) => {
                let msg = format!("warn: batch upsert failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
            }
        }
    }

    let written = batch.len();
    let summary = format!("[PIPELINE plan] wrote {written} roadmap items");
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    0
}

async fn status_async(db: &kavach_db::DbStore, project_slug: &str) -> i32 {
    let project = match kavach_db::project_get_by_slug(db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let msg = format!("error: project not found: {project_slug}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if let Err(code) = crate::cmd::db::validate_project_workdir(&project) {
        return code;
    }
    let Some(project_id) = project.id else {
        if let Err(io_err) = ewrite_or_exit("error: project has no id") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let entries = match kavach_db::list_by_project(&db, "roadmap", &project_id).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let head = format!("[PIPELINE_STATUS] project={project_slug}");
    if let Err(io_err) = print_or_exit(&head) {
        return into_exit_code(io_err);
    }
    let mut printed = false;
    for status_label in ["in_progress", "todo", "done"] {
        let group: Vec<&kavach_db::MemoryEntry> = entries
            .iter()
            .filter(|e| e.entry_status_str() == status_label)
            .collect();
        if group.is_empty() {
            continue;
        }
        printed = true;
        let group_line = format!("  [{status_label}] ({} item(s))", group.len());
        if let Err(io_err) = print_or_exit(&group_line) {
            return into_exit_code(io_err);
        }
        for e in &group {
            let title_first_line = e.title.lines().next().map_or(e.title.as_str(), |line| line);
            let item_line = format!("    - {} — {}", e.entry_key, title_first_line);
            if let Err(io_err) = print_or_exit(&item_line) {
                return into_exit_code(io_err);
            }
        }
    }
    if !printed && let Err(io_err) = print_or_exit("  no open roadmap items — all verified") {
        return into_exit_code(io_err);
    }
    0
}
