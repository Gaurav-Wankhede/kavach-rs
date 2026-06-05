use std::fs;

use kavach_rag_core::RagTree;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

pub(crate) fn load_tree(path: &str) -> Result<RagTree, i32> {
    let body = match fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) => {
            let msg = format!("read '{path}' failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(io_err));
            }
            return Err(1);
        }
    };
    match RagTree::from_json(&body) {
        Ok(t) => Ok(t),
        Err(e) => {
            let msg = format!("parse '{path}' failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(io_err));
            }
            Err(1)
        }
    }
}
