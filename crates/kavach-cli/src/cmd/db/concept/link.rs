use super::common::{emit_err, emit_ok};
use crate::cmd::db::rpc_client;

pub(crate) fn run(from: &str, edge: &str, to: &str) -> i32 {
    match rpc_client::concept_link(from, edge, to) {
        Ok(_) => emit_ok(&format!("linked: {from} -{edge}-> {to}")),
        Err(e) => emit_err(&format!("link: {e}")),
    }
}
