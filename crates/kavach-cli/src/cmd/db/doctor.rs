//! `kavach db doctor --repair-deps` — collapse the duplicate `DEPENDS_ON:` lines
//! the two mirrors minted.
//!
//! The writer is fixed first (`mirror_depends_on_into_content` now compares by
//! bare key, so no NEW duplicate is minted). This repairs what already exists.
//!
//! NON-DESTRUCTIVE: dependencies are MERGED, never dropped. A row can carry more
//! targets on its qualified line than on its bare one — real example
//! `model-route-autoswitch-and-category` — so "delete the qualified line" would
//! silently destroy a live edge. `--dry-run` is the default posture: print the
//! plan, write nothing.
//!
//! IDEMPOTENT: a second run reports zero changes.

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::repair_dep_lines::repair_dep_lines;
use kavach_db::turso::entries::EntryStore;

fn fetch_roadmap(project: &str) -> Result<Vec<kavach_db::MemoryEntry>, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let store = crate::cmd::db::rpc_client::open_turso_resilient().await?;
        store
            .list_by_category(project, "roadmap")
            .await
            .map_err(|e| e.to_string())
    })
}

pub(crate) fn repair_deps(project: &str, apply: bool) -> i32 {
    let rows = match fetch_roadmap(project) {
        Ok(r) => r,
        Err(e) => {
            return match ewrite_or_exit(&format!("error: {e}")) {
                Ok(()) => 1,
                Err(io_err) => into_exit_code(io_err),
            };
        }
    };

    let mut planned: Vec<(kavach_db::MemoryEntry, String, usize, usize)> = Vec::new();
    for row in &rows {
        let (new_body, report) = repair_dep_lines(&row.entry_key, &row.content);
        if report.changed {
            planned.push((row.clone(), new_body, report.deps_before, report.deps_after));
        }
    }

    if planned.is_empty() {
        return match print_or_exit("no duplicate DEPENDS_ON lines found — nothing to repair") {
            Ok(()) => 0,
            Err(io_err) => into_exit_code(io_err),
        };
    }

    let verb = if apply { "repairing" } else { "would repair" };
    let header = format!(
        "{verb} {} row(s) with duplicate DEPENDS_ON lines:",
        planned.len()
    );
    if let Err(io_err) = ewrite_or_exit(&header) {
        return into_exit_code(io_err);
    }
    for (row, _, before, after) in &planned {
        let line = format!(
            "  {} — {} dep(s) -> {}",
            row.entry_key, before, after
        );
        if let Err(io_err) = ewrite_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }

    if !apply {
        let hint = "dry run — re-run with --apply to write (backup first)";
        return match ewrite_or_exit(hint) {
            Ok(()) => 0,
            Err(io_err) => into_exit_code(io_err),
        };
    }

    // Write through the typed RPC path so the single-writer invariant holds.
    let mut written = 0usize;
    for (row, new_body, _, _) in &planned {
        let code = crate::cmd::db::write::run(&crate::cmd::db::rpc_client::WriteRequest {
            project,
            category: "roadmap",
            key: &row.entry_key,
            title: &row.title,
            content: Some(new_body.as_str()),
            new: false,
            update_key: Some(row.entry_key.as_str()),
            priority: row.priority,
            exec_prompt: row.exec_prompt.as_deref(),
            depends_on: &[],
        });
        if code == 0 {
            written = written.saturating_add(1);
        }
    }
    match print_or_exit(&format!("repaired {written} row(s)")) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}