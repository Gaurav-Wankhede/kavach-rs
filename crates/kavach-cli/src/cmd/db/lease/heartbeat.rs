// SPEC: docs/architecture/session-occupancy-lease.md
// `kavach db lease heartbeat --table T --key K --session SID --epoch N --expires DT` — renew TTL.
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://docs.rs/jsonrpsee/0.24
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use chrono::{DateTime, Utc};
use kavach_rpc::client::call;
use kavach_rpc::methods::lease::{HeartbeatParams, HeartbeatResult};

pub fn run(table: &str, key: &str, session_id: &str, epoch: i64, expires_at: DateTime<Utc>) -> i32 {
    let params = HeartbeatParams {
        table: table.to_string(),
        key: key.to_string(),
        session_id: session_id.to_string(),
        epoch,
        expires_at,
    };
    match call::<_, HeartbeatResult>("lease.heartbeat", Some(params)) {
        Ok(r) => {
            let line = format!("renewed: expires={}", r.expires_at);
            match print_or_exit(&line) {
                Ok(()) => 0,
                Err(io) => into_exit_code(io),
            }
        }
        Err(e) => {
            let line = format!("error: lease preempted or rpc failed: {e}");
            match ewrite_or_exit(&line) {
                Ok(()) => 1,
                Err(io) => into_exit_code(io),
            }
        }
    }
}
