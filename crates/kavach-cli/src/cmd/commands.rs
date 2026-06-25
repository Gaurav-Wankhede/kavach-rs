use crate::cli::{help_md, help_tree};
use crate::cmd::io_safe::{into_exit_code, print_or_exit};

/// Print the command tree (default) or the full Markdown reference.
pub(crate) fn run(tree: bool, markdown: bool) -> i32 {
    let body = if markdown {
        help_md::render()
    } else {
        let _ = tree;
        help_tree::render()
    };
    match print_or_exit(body.trim_end()) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
