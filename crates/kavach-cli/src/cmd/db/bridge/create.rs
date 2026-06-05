use super::common::{emit_err, emit_ok};
use crate::cmd::db::rpc_client;

pub(crate) fn run(src_table: &str, src_key: &str, edge: &str, concept: &str) -> i32 {
    match rpc_client::bridge_create(src_table, src_key, edge, concept) {
        Ok(r) => emit_ok(&format!(
            "bridged: {src_table}/{src_key} -{edge}-> concept:{concept} (id={})",
            r.id
        )),
        Err(e) => emit_err(&format!("bridge: {e}")),
    }
}
