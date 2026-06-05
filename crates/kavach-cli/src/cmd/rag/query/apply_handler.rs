use std::fs;

use kavach_rag_core::{SummaryResponse, apply_summaries, pending_requests};

use super::loader::load_tree;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn handle_apply(tree_path: &str, responses_path: &str) -> i32 {
    let mut tree = match load_tree(tree_path) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let body = match fs::read_to_string(responses_path) {
        Ok(b) => b,
        Err(e) => {
            let msg = format!("read '{responses_path}' failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let mut responses: Vec<SummaryResponse> = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SummaryResponse>(line) {
            Ok(r) => responses.push(r),
            Err(e) => {
                let msg = format!("parse response line {} failed: {e}", idx.saturating_add(1));
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        }
    }
    apply_summaries(&mut tree.root, &responses);
    let json = match tree.to_json_pretty() {
        Ok(j) => j,
        Err(e) => {
            let msg = format!("serialize failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    if let Err(io_err) = print_or_exit(&json) {
        return into_exit_code(io_err);
    }
    0
}

pub(crate) fn handle_pending(tree_path: &str) -> i32 {
    let tree = match load_tree(tree_path) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let reqs = pending_requests(&tree.root);
    for req in &reqs {
        let line = match req.to_line() {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("serialize failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    0
}
