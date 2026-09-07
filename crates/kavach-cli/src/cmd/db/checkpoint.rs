// `kavach db checkpoint` — WAL checkpoint operations on the embedded database.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(truncate: bool) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    runtime.block_on(async {
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        if let Err(e) = store.wal_checkpoint(truncate).await {
            let msg = format!("error: checkpoint failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }

        let mode = if truncate { "TRUNCATE" } else { "PASSIVE" };
        let msg = format!("WAL checkpoint completed ({mode})");
        if let Err(io_err) = print_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        0
    })
}
