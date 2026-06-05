use super::common::{emit_err, emit_ok, split_csv};
use crate::cmd::db::rpc_client;

pub(crate) fn run(
    name: &str,
    display: &str,
    desc: &str,
    tags_csv: Option<&str>,
    sources_csv: Option<&str>,
) -> i32 {
    let tags = split_csv(tags_csv);
    let sources = split_csv(sources_csv);
    match rpc_client::concept_add(name, display, desc, tags, sources) {
        Ok(r) => emit_ok(&format!("concept upserted: {name} (id={})", r.id)),
        Err(e) => emit_err(&format!("upsert: {e}")),
    }
}
