use super::common::{emit_concept_rows, emit_err};
use crate::cmd::db::rpc_client;

pub(crate) fn run(limit: usize) -> i32 {
    match rpc_client::concept_list(limit) {
        Ok(rows) => emit_concept_rows(&rows),
        Err(e) => emit_err(&format!("list: {e}")),
    }
}
