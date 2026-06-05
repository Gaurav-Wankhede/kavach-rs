// split: intentional - single command handler for wipe-project
use kavach_rpc::methods::db::wipe_confirm_phrase;
use kavach_surreal::{open_default, wipe::preview_wipe, wipe_project};

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit, prompt_line_or_exit};

#[expect(
    clippy::too_many_lines,
    reason = "single command handler: RPC attempt, fallback path, and async runtime management must stay unified"
)]
pub(super) fn run(project: &str, confirm: bool, dry_run: bool) -> i32 {
    if dry_run {
        return run_dry_run(project);
    }

    if !confirm {
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
        let db = match open_default().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: failed to open db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        match wipe_project(&db, project).await {
            Ok(report) => {
                let head = format!("[wipe] project: {}", report.project_slug);
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                let mut total = 0usize;
                for (table, count) in &report.tables {
                    if *count > 0 {
                        let line = format!("[wipe] {table}: removed {count} rows");
                        if let Err(io_err) = print_or_exit(&line) {
                            return into_exit_code(io_err);
                        }
                    }
                    total = total.saturating_add(*count);
                }
                if report.project_deleted
                    && let Err(io_err) = print_or_exit("[wipe] project registry: removed 1 row")
                {
                    return into_exit_code(io_err);
                }
                let summary = format!(
                    "[wipe] total: {total} rows removed across {} tables",
                    report.tables.len()
                );
                if let Err(io_err) = print_or_exit(&summary) {
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

fn run_dry_run(project: &str) -> i32 {
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
        let db = match open_default().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: failed to open db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        match preview_wipe(&db, project).await {
            Ok(report) => {
                let head = format!("[dry-run] project: {}", report.project_slug);
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                let mut total = 0usize;
                for (table, count) in &report.tables {
                    if *count > 0 {
                        let line = format!("[dry-run] {table}: would remove {count} rows");
                        if let Err(io_err) = print_or_exit(&line) {
                            return into_exit_code(io_err);
                        }
                    }
                    total = total.saturating_add(*count);
                }
                if let Err(io_err) = print_or_exit("[dry-run] project registry: would remove 1 row")
                {
                    return into_exit_code(io_err);
                }
                let summary = format!(
                    "[dry-run] total: {total} rows would be removed across {} tables",
                    report.tables.len()
                );
                if let Err(io_err) = print_or_exit(&summary) {
                    return into_exit_code(io_err);
                }
                if let Err(io_err) =
                    print_or_exit("[dry-run] NO CHANGES MADE — run with --confirm to execute")
                {
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
