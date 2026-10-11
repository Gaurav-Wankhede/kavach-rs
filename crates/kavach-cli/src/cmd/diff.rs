use ast_grep_core::AstGrep;
use serde_json::json;
use similar::{ChangeTag, TextDiff};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn run(
    path_a: &str,
    path_b: &str,
    stat: bool,
    brief: bool,
    symbols: bool,
    context: usize,
) -> i32 {
    let p_a = Path::new(path_a);
    let p_b = Path::new(path_b);

    if !p_a.exists() {
        eprintln!("kavach diff: path '{}' does not exist", path_a);
        return 2;
    }
    if !p_b.exists() {
        eprintln!("kavach diff: path '{}' does not exist", path_b);
        return 2;
    }

    let content_a = match fs::read_to_string(p_a) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach diff: cannot read '{}': {}", path_a, e);
            return 2;
        }
    };
    let content_b = match fs::read_to_string(p_b) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach diff: cannot read '{}': {}", path_b, e);
            return 2;
        }
    };

    if content_a == content_b {
        if brief {
            println!("Files are identical");
        } else if stat {
            println!("0 lines changed (+0, -0)");
        }
        return 0;
    }

    if brief {
        println!("Files differ: {} vs {}", path_a, path_b);
        return 1;
    }

    let diff = TextDiff::from_lines(&content_a, &content_b);

    if symbols {
        let lang = crate::cmd::trace::resolve_lang(path_b)
            .or_else(|| crate::cmd::trace::resolve_lang(path_a));

        if let Some(l) = lang {
            let mut modified_lines_b = BTreeSet::new();
            let mut current_line_b = 1usize;

            for change in diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Equal => {
                        current_line_b += 1;
                    }
                    ChangeTag::Delete => {}
                    ChangeTag::Insert => {
                        modified_lines_b.insert(current_line_b);
                        current_line_b += 1;
                    }
                }
            }

            let grep = AstGrep::new(&content_b, l);
            let root = grep.root();
            let mut symbol_hits = Vec::new();

            for node in root.dfs() {
                if let Some((kind, name)) = crate::cmd::trace::any_definition_node(&node) {
                    let range = node.range();
                    let (start_line, _) = crate::cmd::trace::offset_to_line_and_col(&content_b, range.start);
                    let (end_line, _) = crate::cmd::trace::offset_to_line_and_col(&content_b, range.end);

                    let intersects = (start_line..=end_line).any(|line_num| modified_lines_b.contains(&line_num));
                    if intersects {
                        symbol_hits.push(json!({
                            "kind": kind,
                            "symbol": name,
                            "loc": format!("{}:{}:{}", path_b, start_line, 1),
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
    }

    if stat {
        let mut insertions = 0usize;
        let mut deletions = 0usize;
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => insertions += 1,
                ChangeTag::Delete => deletions += 1,
                ChangeTag::Equal => {}
            }
        }
        println!("Diff summary: +{} insertions, -{} deletions", insertions, deletions);
        return 0;
    }

    // Unified diff
    let unified = diff
        .unified_diff()
        .context_radius(context)
        .header(path_a, path_b)
        .to_string();

    print!("{}", unified);
    0
}
