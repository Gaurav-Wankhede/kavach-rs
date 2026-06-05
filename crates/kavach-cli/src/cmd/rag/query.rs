mod apply_handler;
mod loader;

use kavach_rag_core::{Matcher, Query};

use crate::cmd::io_safe::{into_exit_code, print_or_exit};

// Re-export to hub
pub(super) use apply_handler::{handle_apply, handle_pending};

pub(super) fn handle_query(
    tree_path: &str,
    file: &str,
    text: &str,
    intent: &str,
    top_k: usize,
) -> i32 {
    let tree = match loader::load_tree(tree_path) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let matcher = Matcher::new(&tree).with_top_k(top_k);
    let query = Query::new(file, text, intent);
    let hits = matcher.run(&query);
    for hit in &hits {
        let line = format!("{}\t{}\t{}", hit.score.0, hit.node_id, hit.title);
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    0
}
