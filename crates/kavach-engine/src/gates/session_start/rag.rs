//! RAG tree refresh at session start (silent, never blocks boot).

/// Refresh all registered RAG trees on session start. Queries kavach-db
/// for labels with a stored `source_dir`, then runs `kavach rag refresh-if-stale`
/// for each. Falls back to skills-only if the DB query fails.
/// Failure is silent — the gate must never block session init on RAG maintenance.
pub(super) fn refresh_all_rag_trees() {
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call::<(), serde_json::Value>("rag.tree_list_refreshable", None);
    let arr = match result {
        Ok(serde_json::Value::Array(a)) if !a.is_empty() => a,
        _ => {
            refresh_single_tree("skills", &kavach_config::skills_dir());
            return;
        }
    };
    for v in &arr {
        let Some(label) = v.get("source").and_then(|s| s.as_str()) else {
            continue;
        };
        let Some(source_dir) = v.get("source_dir").and_then(|s| s.as_str()) else {
            continue;
        };
        let path = std::path::PathBuf::from(source_dir);
        refresh_single_tree(label, &path);
    }
}

fn refresh_single_tree(label: &str, source_dir: &std::path::Path) {
    let Some(source) = source_dir.to_str() else {
        return;
    };
    std::process::Command::new("kavach")
        .args([
            "rag",
            "refresh-if-stale",
            "--source",
            source,
            "--label",
            label,
        ])
        .output()
        .ok();
}
