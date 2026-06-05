use super::common::{emit_err, emit_ok};
use crate::cmd::db::rpc_client;

pub(crate) fn run(name: &str) -> i32 {
    match rpc_client::concept_delete(name) {
        Ok(n) => emit_ok(&format!("removed: {n} row(s) for concept '{name}'")),
        Err(e) => emit_err(&format!("removal failed: {e}")),
    }
}

pub(crate) fn run_by_prefix(prefix: &str, confirm: bool) -> i32 {
    if !confirm {
        return emit_err("--confirm required for bulk prefix purge");
    }
    match rpc_client::concept_delete_by_prefix(prefix, confirm) {
        Ok(n) => emit_ok(&format!("removed: {n} row(s) for prefix '{prefix}'")),
        Err(e) => emit_err(&format!("prefix purge failed: {e}")),
    }
}
