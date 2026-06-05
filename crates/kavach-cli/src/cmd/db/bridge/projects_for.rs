use super::common::emit_err;
use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use kavach_surreal::ProjectHit;

fn render(hits: &[ProjectHit]) -> i32 {
    if hits.is_empty() {
        return match print_or_exit("no projects reference this concept") {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        };
    }
    for h in hits {
        let line = format!(
            "  {} via {}/{} -{}->",
            h.project_slug, h.src_table, h.src_key, h.edge
        );
        if let Err(io) = print_or_exit(&line) {
            return into_exit_code(io);
        }
    }
    0
}

pub(crate) fn run(concept_name: &str) -> i32 {
    match rpc_client::bridge_projects_for(concept_name) {
        Ok(hits) => render(&hits),
        Err(e) => emit_err(&format!("query: {e}")),
    }
}
