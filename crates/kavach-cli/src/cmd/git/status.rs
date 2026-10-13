use serde_json::json;

pub(crate) fn run() -> i32 {
    let repo = match gix::open(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let status_platform = match repo.status(gix::progress::Discard) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach git: failed to inspect status: {}", e);
            return 1;
        }
    };

    let mut changed = Vec::new();

    if let Ok(iter) = status_platform.into_index_worktree_iter(Vec::new()) {
        for item_res in iter {
            if let Ok(item) = item_res {
                let path = item.rela_path().to_string();
                let status_str = match item.summary() {
                    Some(s) => format!("{:?}", s),
                    None => "M".to_string(),
                };
                let clean_status = if status_str.contains("Modified") {
                    "M"
                } else if status_str.contains("Added") {
                    "A"
                } else if status_str.contains("Removed") {
                    "D"
                } else {
                    "M"
                };
                changed.push(json!({
                    "path": path,
                    "status": clean_status,
                }));
            }
        }
    }

    if changed.is_empty() {
        println!("working tree clean");
    } else {
        let tree_input: Vec<(&str, Option<&str>)> = changed
            .iter()
            .filter_map(|item| {
                let p = item.get("path")?.as_str()?;
                let s = item.get("status")?.as_str()?;
                Some((p, Some(s)))
            })
            .collect();
        let toon = kavach_toon::encode_path_tree(tree_input);
        println!("{}", toon.trim_end());
    }

    0
}
