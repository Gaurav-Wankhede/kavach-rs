// SPEC: docs/architecture/session-occupancy-lease.md
// `kavach db lease acquire --table T --key K --session SID` — try to claim the lease.
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://docs.rs/jsonrpsee/0.24
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_rpc::client::call;
use kavach_rpc::methods::lease::{AcquireParams, AcquireResult};

pub fn run(table: &str, key: &str, session_id: &str) -> i32 {
    let params = AcquireParams {
        table: table.to_string(),
        key: key.to_string(),
        session_id: session_id.to_string(),
    };
    match call::<_, AcquireResult>("lease.acquire", Some(params)) {
        Ok(r) if r.acquired => emit(&format!(
            "acquired: {} epoch={} expires={}",
            r.session_id,
            match r.epoch { Some(e) => e.to_string(), None => "?".to_string() },
            r.expires_at
        )),
        Ok(r) => emit_err(&format!("held by {}: expires={}", r.session_id, r.expires_at)),
        Err(e) => emit_err(&format!("rpc: {e}")),
    }
}

fn emit(msg: &str) -> i32 {
    match print_or_exit(msg) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

fn emit_err(msg: &str) -> i32 {
    let line = format!("error: {msg}");
    match ewrite_or_exit(&line) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}
