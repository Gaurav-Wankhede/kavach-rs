// SPEC: docs/architecture/session-occupancy-lease.md
// `kavach db lease unlock --table T --key K --session SID --epoch N --expires DT` — clear lease fields.
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
// SOURCE: https://docs.rs/jsonrpsee/0.24
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use chrono::{DateTime, Utc};
use kavach_rpc::client::call;
use kavach_rpc::methods::lease::{UnlockParams, UnlockResult};

pub fn run(table: &str, key: &str, session_id: &str, epoch: i64, expires_at: DateTime<Utc>) -> i32 {
    let params = UnlockParams {
        table: table.to_string(),
        key: key.to_string(),
        session_id: session_id.to_string(),
        epoch,
        expires_at,
    };
    match call::<_, UnlockResult>("lease.unlock", Some(params)) {
        Ok(_) => match print_or_exit("unlocked") {
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
