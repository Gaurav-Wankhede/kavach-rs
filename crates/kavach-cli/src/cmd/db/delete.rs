// split: intentional - single command handler for granular delete
use kavach_rpc::methods::db::delete_confirm_phrase;
use kavach_surreal::{
    delete_by_key, delete_category, open_default, preview_delete_by_key, preview_delete_category,
};

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit, prompt_line_or_exit};

#[expect(
    clippy::too_many_lines,
    reason = "single command handler with intentional control flow for granular delete operations"
)]
pub(super) fn run(
    project: &str,
    category: &str,
    key: Option<&str>,
    all: bool,
    confirm: bool,
    dry_run: bool,
) -> i32 {
    if key.is_none() && !all {
        if let Err(io_err) = ewrite_or_exit("error: must specify --key <key> or --all") {
            return into_exit_code(io_err);
        }
        return 1;
    }
    if key.is_some() && all {
        if let Err(io_err) = ewrite_or_exit("error: cannot use both --key and --all") {
            return into_exit_code(io_err);
        }
        return 1;
    }
    if all && !confirm && !dry_run {
        if let Err(io_err) =
            ewrite_or_exit("error: --all requires --confirm (or use --dry-run to preview)")
        {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // Typed-confirmation gate. A real (non-dry-run) delete requires the exact
    // target-bound phrase to be typed at the prompt; the daemon re-checks it,
    // so this can't be skipped by editing the CLI. dry-run never prompts.
    let confirm_phrase: Option<String> = if dry_run {
        None
    } else {
        let expected = delete_confirm_phrase(project, category, key);
        let prompt =
            format!("This permanently deletes data. To proceed, type exactly:\n  {expected}\n> ");
        let typed = match prompt_line_or_exit(&prompt) {
            Ok(t) => t,
            Err(io_err) => return into_exit_code(io_err),
        };
        if typed != expected {
            if let Err(io_err) =
                ewrite_or_exit("error: confirmation phrase did not match — aborted")
            {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Some(typed)
    };

    match super::rpc_client::delete(project, category, key, all, dry_run, confirm_phrase) {
        Ok(result) if result.success => {
            let prefix = if result.dry_run {
                "(dry-run)"
            } else {
                "(applied)"
            };
            let n = result.deleted_count;
            let msg = format!("{prefix} {n} row(s) in [{category}] for project '{project}'");
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(result) => {
            let err_text = result.error.unwrap_or_else(|| "unknown".to_owned());
            let msg = format!("error: {err_text}");
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

        if let Some(k) = key {
            if dry_run {
                match preview_delete_by_key(&db, project, category, k).await {
                    Ok(report) => {
                        let l1 = format!("(dry-run) target: {category}/{k}");
                        if let Err(io_err) = print_or_exit(&l1) {
                            return into_exit_code(io_err);
                        }
                        let l2 = format!("(dry-run) matches: {} row(s)", report.count);
                        if let Err(io_err) = print_or_exit(&l2) {
                            return into_exit_code(io_err);
                        }
                        if let Err(io_err) = print_or_exit("(dry-run) NO CHANGES MADE") {
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
            } else {
                match delete_by_key(&db, project, category, k).await {
                    Ok(report) => {
                        let msg = format!(
                            "(applied) {}/{} {} row(s)",
                            report.category, k, report.count
                        );
                        if let Err(io_err) = print_or_exit(&msg) {
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
            }
        } else {
            if dry_run {
                match preview_delete_category(&db, project, category).await {
                    Ok(report) => {
                        let l1 = format!("(dry-run) target: ALL rows in {category}");
                        if let Err(io_err) = print_or_exit(&l1) {
                            return into_exit_code(io_err);
                        }
                        let l2 = format!("(dry-run) count: {} row(s)", report.count);
                        if let Err(io_err) = print_or_exit(&l2) {
                            return into_exit_code(io_err);
                        }
                        if let Err(io_err) = print_or_exit(
                            "(dry-run) NO CHANGES MADE — run with --confirm to execute",
                        ) {
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
            } else {
                match delete_category(&db, project, category).await {
                    Ok(report) => {
                        let msg =
                            format!("(applied) all {} {} row(s)", report.category, report.count);
                        if let Err(io_err) = print_or_exit(&msg) {
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
            }
        }
    })
}
