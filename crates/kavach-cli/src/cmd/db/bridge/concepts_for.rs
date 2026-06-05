use super::common::emit_err;
use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use kavach_surreal::BridgeHit;

fn render(hits: &[BridgeHit]) -> i32 {
    if hits.is_empty() {
        return match print_or_exit("no bridges from this project") {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        };
    }
    for h in hits {
        let line = format!(
            "  {}/{} -{}-> {}",
            h.src_table, h.src_key, h.edge, h.concept.name
        );
        if let Err(io) = print_or_exit(&line) {
            return into_exit_code(io);
        }
    }
    0
}

pub(crate) fn run(project_slug: &str) -> i32 {
    match rpc_client::bridge_concepts_for(project_slug) {
        Ok(hits) => render(&hits),
        Err(e) => emit_err(&format!("query: {e}")),
    }
}
