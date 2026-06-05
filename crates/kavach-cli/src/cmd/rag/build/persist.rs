use kavach_rag_core::RagTree;

use super::super::util::{hash_bytes, short_hash};
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn persist_trees(label: &str, trees: &[RagTree], source_dir: &str) -> i32 {
    let mut payload = String::new();
    for tree in trees {
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
        payload.push_str(&json);
        payload.push('\n');
    }
    let built_at = kavach_hook::today();
    let source_hash = hash_bytes(payload.as_bytes());
    let short = short_hash(&source_hash);
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
        if let Err(e) = kavach_surreal::rag_tree_upsert_with_dir(
            &db,
            label,
            &built_at,
            payload.as_bytes(),
            &source_hash,
            source_dir,
        )
        .await
        {
            let msg = format!("db upsert failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let ok_line = format!(
            "persisted {} tree(s) under label '{label}' (hash {short})",
            trees.len()
        );
        if let Err(io_err) = print_or_exit(&ok_line) {
            return into_exit_code(io_err);
        }
        0
    })
}
