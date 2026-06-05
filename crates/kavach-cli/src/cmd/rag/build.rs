mod persist;
mod refresh;

use std::path::PathBuf;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

// Re-export to hub
pub(super) use persist::persist_trees;
pub(super) use refresh::handle_refresh_if_stale;

pub(super) fn handle_build(source: &str, label: &str, persist_flag: bool) -> i32 {
    let path = PathBuf::from(source);
    let trees = match kavach_rag_core::build_trees_from_dir(&path, label) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("rag build failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if persist_flag {
        return persist_trees(label, &trees, source);
    }
    for tree in &trees {
        let json = match tree.to_json() {
            Ok(j) => j,
            Err(e) => {
                let msg = format!("serialize failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(io_err) = print_or_exit(&json) {
            return into_exit_code(io_err);
        }
    }
    0
}

pub(super) fn handle_list() -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async {
        let db = match kavach_surreal::open_default().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("db open failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        let rows = match kavach_surreal::rag_tree_list(&db).await {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("db list failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        for row in &rows {
            let short = super::util::short_hash(&row.source_hash);
            let line = format!("{}\t{}\t{short}", row.source, row.built_at);
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
        0
    })
}
