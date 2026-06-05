use std::path::PathBuf;

use super::super::util::{filter_and_enrich_trees, hash_bytes, serialize_payload, short_hash};
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn handle_refresh_if_stale(source: &str, label: &str) -> i32 {
    let path = PathBuf::from(source);
    let all_trees = match kavach_rag_core::build_trees_from_dir(&path, label) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("scan failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let trees = filter_and_enrich_trees(all_trees, label, &path);
    let prospective = match serialize_payload(&trees) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let prospective_hash = hash_bytes(prospective.as_bytes());
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
        let current = match kavach_surreal::rag_tree_get(&db, label).await {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("db get failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Some(stored) = current
            && stored.source_hash == prospective_hash
        {
            let fresh = format!(
                "rag '{label}' fresh (hash {})",
                short_hash(&prospective_hash)
            );
            if let Err(io_err) = print_or_exit(&fresh) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        let built_at = kavach_hook::today();
        if let Err(e) = kavach_surreal::rag_tree_upsert_with_dir(
            &db,
            label,
            &built_at,
            prospective.as_bytes(),
            &prospective_hash,
            source,
        )
        .await
        {
            let msg = format!("db upsert failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let refreshed = format!(
            "rag '{label}' refreshed ({} trees, hash {})",
            trees.len(),
            short_hash(&prospective_hash)
        );
        if let Err(io_err) = print_or_exit(&refreshed) {
            return into_exit_code(io_err);
        }
        0
    })
}
