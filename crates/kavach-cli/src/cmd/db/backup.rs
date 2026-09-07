// `kavach db backup` — file-copy snapshots with integrity check into ~/.kavach/backups/
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_BACKUPS: usize = 10;

pub(super) fn run(keep: usize) -> i32 {
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

        // 1. Flush WAL before copying
        if let Err(e) = store.wal_checkpoint(true).await {
            let msg = format!("error: pre-backup wal_checkpoint failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }

        // 2. Run integrity check
        match store.integrity_check().await {
            Ok(rows) => {
                if rows.len() != 1 || rows.first().is_none_or(|r| r != "ok") {
                    let msg = format!("error: integrity check failed before backup: {rows:?}");
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            }
            Err(e) => {
                let msg = format!("error: integrity check failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        }

        // 3. Resolve paths
        let base = std::env::var_os("KAVACH_HOME").map_or_else(
            || {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".kavach")
            },
            PathBuf::from,
        );
        let db_path = base.join("kavach.db");
        if !db_path.exists() {
            let msg = format!("error: db file not found at {}", db_path.display());
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }

        let backups_dir = base.join("backups");
        if let Err(e) = fs::create_dir_all(&backups_dir) {
            let msg = format!("error: create backups dir {}: {e}", backups_dir.display());
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let dest_filename = format!("kavach_{timestamp}.db");
        let dest_path = backups_dir.join(&dest_filename);

        if let Err(e) = fs::copy(&db_path, &dest_path) {
            let msg = format!("error: copy to {} failed: {e}", dest_path.display());
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }

        let keep_limit = if keep == 0 { MAX_BACKUPS } else { keep };
        rotate_backups(&backups_dir, keep_limit);

        let msg = format!("backup created: {}", dest_path.display());
        if let Err(io_err) = print_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        0
    })
}

fn rotate_backups(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.is_file() && p.extension().is_some_and(|ext| ext == "db")).then_some(p)
        })
        .collect();
    files.sort();
    if files.len() > keep {
        let to_remove = files.len().saturating_sub(keep);
        for p in files.into_iter().take(to_remove) {
            drop(fs::remove_file(p));
        }
    }
}
