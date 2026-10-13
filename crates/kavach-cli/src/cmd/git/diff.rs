use ast_grep_core::AstGrep;
use serde_json::json;


pub(crate) fn run(stat_only: bool, files_only: bool, symbols_only: bool) -> i32 {
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

    let mut modified_files = Vec::new();

    if let Ok(iter) = status_platform.into_index_worktree_iter(Vec::new()) {
        for item_res in iter {
            if let Ok(item) = item_res {
                let path = item.rela_path().to_string();
                modified_files.push(path);
            }
        }
    }

    if files_only {
        let tree_input: Vec<(&str, Option<&str>)> = modified_files
            .iter()
            .map(|f| (f.as_str(), None))
            .collect();
        let toon = kavach_toon::encode_path_tree(tree_input);
        println!("{}", toon.trim_end());
        return 0;
    }

    if stat_only {
        println!("{} files modified", modified_files.len());
        return 0;
    }

    if symbols_only {
        let mut symbol_hits = Vec::new();

        for file_path in &modified_files {
            let lang = match crate::cmd::trace::resolve_lang(file_path) {
                Some(l) => l,
                None => continue,
            };

            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let grep = AstGrep::new(&content, lang);
            let root = grep.root();

            for node in root.dfs() {
                if let Some((kind, name)) = crate::cmd::trace::any_definition_node(&node) {
                    let range = node.range();
                    let (start_line, _) = crate::cmd::trace::offset_to_line_and_col(&content, range.start);
                    symbol_hits.push(json!({
                        "kind": kind,
                        "symbol": name,
                        "loc": format!("{}:{}:{}", file_path, start_line, 1),
                    }));
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

    if !modified_files.is_empty() {
        let tree_input: Vec<(&str, Option<&str>)> = modified_files
            .iter()
            .map(|f| (f.as_str(), Some("M")))
            .collect();
        let toon = kavach_toon::encode_path_tree(tree_input);
        println!("{}", toon.trim_end());
    }
    println!("── Summary: {} files modified ──", modified_files.len());

    0
}
