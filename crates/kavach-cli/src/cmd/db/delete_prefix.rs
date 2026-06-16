// split: intentional — prefix-bound bulk purge, sibling of granular `delete`.
//! `kavach db delete-prefix` — bulk-delete every record in a category whose key
//! starts with a prefix (e.g. clear all `heal.incident.loophole-*` cards).
use kavach_rpc::methods::db::delete_confirm_phrase_prefix;
use kavach_surreal::{delete_by_key_prefix, open_default, preview_delete_by_key_prefix};

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit, prompt_line_or_exit};

/// Format the result line shared by the RPC and direct-DB paths.
fn ok_line(tag: &str, count: usize, category: &str, prefix: &str, project: &str) -> i32 {
    let msg = format!("{tag} {count} {category} row(s) matching '{prefix}*' in '{project}'");
    match print_or_exit(&msg) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}

/// Emit an error line and return exit code 1 (or the IO failure's code).
fn err_line(text: &str) -> i32 {
    if let Err(io_err) = ewrite_or_exit(text) {
        return into_exit_code(io_err);
    }
    1
}

/// Bulk-purge records in `category` whose key starts with `prefix`. A real run
/// requires the typed prefix-confirmation phrase (re-checked by the daemon, so it
/// can't be skipped by editing the CLI); dry-run only counts the matches.
pub(super) fn run(project: &str, category: &str, prefix: &str, confirm: bool, dry_run: bool) -> i32 {
    if prefix.is_empty() {
        return err_line("error: --prefix must not be empty");
    }
    if !confirm && !dry_run {
        return err_line("error: prefix purge requires --confirm (or use --dry-run to preview)");
    }

    let confirm_phrase = match confirm_phrase(project, category, prefix, dry_run) {
        Ok(p) => p,
        Err(code) => return code,
    };

    match super::rpc_client::delete_by_key_prefix(project, category, prefix, dry_run, confirm_phrase)
    {
        Ok(result) if result.success => {
            let tag = if result.dry_run { "(dry-run)" } else { "(applied)" };
            ok_line(tag, result.deleted_count, category, prefix, project)
        }
        Ok(result) => err_line(&format!(
            "error: {}",
            result.error.unwrap_or_else(|| "unknown".to_owned())
        )),
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {
            run_direct(project, category, prefix, dry_run)
        }
        Err(e) => err_line(&format!("rpc error: {e}")),
    }
}

/// Prompt for and validate the typed confirm phrase; `Ok(None)` under dry-run.
/// `Err(code)` carries an early-exit code on mismatch or IO failure.
fn confirm_phrase(
    project: &str,
    category: &str,
    prefix: &str,
    dry_run: bool,
) -> Result<Option<String>, i32> {
    if dry_run {
        return Ok(None);
    }
    let expected = delete_confirm_phrase_prefix(project, category, prefix);
    let prompt = format!(
        "This permanently bulk-deletes every {category} record matching '{prefix}*'.\n\
         To proceed, type exactly:\n  {expected}\n> "
    );
    let typed = prompt_line_or_exit(&prompt).map_err(into_exit_code)?;
    if typed != expected {
        return Err(err_line("error: confirmation phrase did not match — aborted"));
    }
    Ok(Some(typed))
}

/// Direct-DB fallback when the RPC daemon is unreachable.
fn run_direct(project: &str, category: &str, prefix: &str, dry_run: bool) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return err_line(&format!("error: tokio runtime: {e}")),
    };
    runtime.block_on(async {
        let db = match open_default().await {
            Ok(d) => d,
            Err(e) => return err_line(&format!("error: failed to open db: {e}")),
        };
        let outcome = if dry_run {
            preview_delete_by_key_prefix(&db, project, category, prefix).await
        } else {
            delete_by_key_prefix(&db, project, category, prefix).await
        };
        match outcome {
            Ok(report) => {
                let tag = if dry_run { "(dry-run)" } else { "(applied)" };
                ok_line(tag, report.count, category, prefix, project)
            }
            Err(e) => err_line(&format!("error: {e}")),
        }
    })
}
