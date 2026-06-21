use crate::cmd::db::rpc_client;
use crate::cmd::io_safe::{err_exit, into_exit_code, print_or_exit};

/// Run a read-only `SurrealQL` query and print the JSON result. The read-only
/// guard (SELECT/INFO only) lives server-side; a write verb is refused there.
pub(super) fn run(query: &str) -> i32 {
    match rpc_client::raw_query(query) {
        Ok(r) => match print_or_exit(&r.json) {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        },
        Err(e) => err_exit(&format!("query: {e}")),
    }
}
