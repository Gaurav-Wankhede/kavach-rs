// SPEC: docs/architecture/session-occupancy-lease.md
// `kavach db lease status --table T --key K` — read lease state.
// SOURCE: https://surrealdb.com/3.0
// SOURCE: https://docs.rs/jsonrpsee/0.24
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_rpc::client::call;
use kavach_rpc::methods::lease::{StatusParams, StatusResult};

pub fn run(table: &str, key: &str) -> i32 {
    let params = StatusParams { table: table.to_string(), key: key.to_string() };
    match call::<_, StatusResult>("lease.status", Some(params)) {
        Ok(r) if r.held => {
            let sid = match r.session_id { Some(s) => s, None => String::from("?") };
            let ep = match r.epoch { Some(e) => e.to_string(), None => String::from("?") };
            let exp = match r.expires_at { Some(t) => t.to_string(), None => String::from("?") };
            let line = format!("held: {sid} epoch={ep} expires={exp}");
            match print_or_exit(&line) {
                Ok(()) => 0,
                Err(io) => into_exit_code(io),
            }
        }
        Ok(_) => match print_or_exit("free") {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        },
        Err(e) => {
            let line = format!("error: {e}");
            match ewrite_or_exit(&line) {
                Ok(()) => 1,
                Err(io) => into_exit_code(io),
            }
        }
    }
}
