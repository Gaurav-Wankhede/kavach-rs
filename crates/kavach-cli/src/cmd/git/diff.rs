use git2::{Patch, Repository};
use serde_json::json;
use ast_grep_core::AstGrep;
use std::collections::BTreeSet;

pub(crate) fn run(stat_only: bool, files_only: bool, symbols_only: bool) -> i32 {
    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let head = repo.head().and_then(|h| h.peel_to_tree()).ok();
    let diff = match repo.diff_tree_to_workdir_with_index(head.as_ref(), None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("kavach git: failed to compute diff: {}", e);
            return 1;
        }
    };

    let stats = match diff.stats() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach git: failed to compute diff stats: {}", e);
            return 1;
        }
    };

    if files_only {
        let mut files = Vec::new();
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                files.push(path.to_string());
            }
        }
        for f in files {
            println!("{}", f);
        }
        return 0;
    }

    if symbols_only {
        let mut symbol_hits = Vec::new();

        for (idx, _delta) in diff.deltas().enumerate() {
            let patch = match Patch::from_diff(&diff, idx) {
                Ok(Some(p)) => p,
                _ => continue,
            };

            let delta = patch.delta();
            let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
                Some(p) => p.to_string(),
                None => continue,
            };

            let lang = match crate::cmd::trace::resolve_lang(&file_path) {
                Some(l) => l,
                None => continue,
            };

            // Collect modified line ranges in the new file
            let mut modified_lines = BTreeSet::new();
            let num_hunks = patch.num_hunks();
            for h_idx in 0..num_hunks {
                if let Ok((hunk, _)) = patch.hunk(h_idx) {
                    let start = hunk.new_start() as usize;
                    let count = hunk.new_lines() as usize;
                    for l in start..(start + count) {
                        modified_lines.insert(l);
                    }
                }
            }

            if modified_lines.is_empty() {
                continue;
            }

            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let grep = AstGrep::new(&content, lang);
            let root = grep.root();

            for node in root.dfs() {
                if let Some((kind, name)) = crate::cmd::trace::any_definition_node(&node) {
                    let range = node.range();
                    let (start_line, _) = crate::cmd::trace::offset_to_line_and_col(&content, range.start);
                    let (end_line, _) = crate::cmd::trace::offset_to_line_and_col(&content, range.end);

                    let intersects = (start_line..=end_line).any(|line_num| modified_lines.contains(&line_num));
                    if intersects {
                        symbol_hits.push(json!({
                            "kind": kind,
                            "symbol": name,
                            "loc": format!("{}:{}:{}", file_path, start_line, 1),
                        }));
                    }
                }
            }
        }

        if !symbol_hits.is_empty() {
            let v = json!(symbol_hits);
            let toon = kavach_toon::encode_toon(&v);
            println!("{}", toon.trim_end());
        } else {
            println!("symbols[0]{{kind,symbol,loc}}:");
        }
        return 0;
    }

    if stat_only {
        println!("{} files changed, {} insertions(+), {} deletions(-)",
            stats.files_changed(),
            stats.insertions(),
            stats.deletions()
        );
        return 0;
    }

    // Default compact diff summary in TOON
    let mut deltas = Vec::new();
    for delta in diff.deltas() {
        let status = format!("{:?}", delta.status());
        let path = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("").to_string();
        deltas.push(json!({
            "status": status,
            "path": path,
        }));
    }

    if !deltas.is_empty() {
        let v = json!(deltas);
        let toon = kavach_toon::encode_toon(&v);
        println!("{}", toon.trim_end());
    }
    println!("── Summary: {} files changed, +{}, -{} ──", stats.files_changed(), stats.insertions(), stats.deletions());

    0
}
