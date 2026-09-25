// split: intentional - single command handler for wipe-project
use kavach_db::turso::entries::EntryStore;
use kavach_rpc::methods::db::wipe_confirm_phrase;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit, prompt_line_or_exit};

#[expect(
    clippy::too_many_lines,
    reason = "single command handler: RPC attempt, fallback path, and async runtime management must stay unified"
)]
pub(super) fn run(project: &str, confirm: bool, dry_run: bool) -> i32 {
    if !confirm && !dry_run {
        if let Err(io_err) =
            ewrite_or_exit("error: --confirm flag required for destructive operation")
        {
            return into_exit_code(io_err);
        }
        if let Err(io_err) =
            ewrite_or_exit("hint: use --dry-run first to preview what will be removed")
        {
            return into_exit_code(io_err);
        }
        let usage = format!("usage: kavach db wipe-project --project {project} --confirm");
        if let Err(io_err) = ewrite_or_exit(&usage) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // Typed-confirmation gate. Wiping a whole project requires typing the exact
    // target-bound phrase; the daemon re-checks it, so the CLI can't bypass it.
    let expected = wipe_confirm_phrase(project);
    let prompt = format!(
        "This permanently wipes the ENTIRE project '{project}'. To proceed, type exactly:\n  {expected}\n> "
    );
    let typed = match prompt_line_or_exit(&prompt) {
        Ok(t) => t,
        Err(io_err) => return into_exit_code(io_err),
    };
    if typed != expected {
        if let Err(io_err) = ewrite_or_exit("error: confirmation phrase did not match — aborted")
        {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let confirm_phrase = Some(typed);

    // RPC-first
    match super::rpc_client::wipe_project(project, false, confirm_phrase) {
        Ok(result) if result.success => {
            if let Some(report) = result.report {
                let head = format!("[wipe] project: {} (via rpc)", report.project_slug);
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                for (table, count) in &report.tables {
                    if *count > 0 {
                        let line = format!("[wipe] {table}: removed {count} rows");
                        if let Err(io_err) = print_or_exit(&line) {
                            return into_exit_code(io_err);
                        }
                    }
                }
            }
            return 0;
        }
        Ok(result) => {
            let err = result.error.unwrap_or_else(|| "unknown".to_owned());
            let msg = format!("error: {err}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            let msg = format!("rpc error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

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
        // Phase 6: direct open via TursoStore (libSQL file — no lock contention).
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        match wipe_project_turso(&store, project, dry_run).await {
            Ok(report) => {
                let prefix = if dry_run { "[dry-run]" } else { "[wipe]" };
                let action = if dry_run { "would remove" } else { "removed" };
                let action_past = if dry_run {
                    "would be removed"
                } else {
                    "removed"
                };
                let head = format!("{} project: {}", prefix, report.project_slug);
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                let mut total = 0usize;
                for (table, count) in &report.tables {
                    if *count > 0 {
                        let line = format!("{prefix} {table}: {action} {count} rows");
                        if let Err(io_err) = print_or_exit(&line) {
                            return into_exit_code(io_err);
                        }
                    }
                    total = total.saturating_add(*count);
                }
                if report.project_deleted {
                    let msg = format!("{prefix} project registry: {action} 1 row");
                    if let Err(io_err) = print_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                }
                let count = report.tables.len();
                let summary =
                    format!("{prefix} total: {total} rows {action_past} across {count} tables");
                if let Err(io_err) = print_or_exit(&summary) {
                    return into_exit_code(io_err);
                }
                if dry_run {
                    let res =
                        print_or_exit("[dry-run] NO CHANGES MADE — run with --confirm to execute");
                    if let Err(io_err) = res {
                        return into_exit_code(io_err);
                    }
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

#[derive(Default)]
struct WipeReport {
    project_slug: String,
    tables: Vec<(String, usize)>,
    project_deleted: bool,
}

async fn wipe_project_turso(
    store: &(impl EntryStore + Sync),
    project: &str,
    dry_run: bool,
) -> Result<WipeReport, String> {
    let mut report = WipeReport {
        project_slug: project.to_owned(),
        tables: Vec::new(),
        project_deleted: false,
    };

    // Categories to wipe
    let categories = [
        "decision", "research", "roadmap", "pattern", "app_spec", "mistake",
    ];

    for cat in categories {
        let rows = store
            .list_by_category(project, cat)
            .await
            .map_err(|e| format!("list {cat}: {e}"))?;
        let count = rows.len();
        if count > 0 {
            report.tables.push((cat.to_owned(), count));
            if !dry_run {
                for row in rows {
                    let key = &row.entry_key;
                    store
                        .delete_by_key(project, cat, key)
                        .await
                        .map_err(|e| format!("delete {cat}/{key}: {e}"))?;
                }
            }
        }
    }

    // TODO: Also wipe events, projects, parts tables (need TursoStore methods)
    // For now, we skip these as they require additional TursoStore methods

    // Project registry deletion would go here
    // project_deleted would be set to true after deleting from projects table

    Ok(report)
}
