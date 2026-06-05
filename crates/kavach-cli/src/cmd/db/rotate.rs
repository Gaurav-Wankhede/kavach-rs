use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(days: i64) -> i32 {
    if days <= 0 {
        if let Err(e) = ewrite_or_exit("error: days must be positive") {
            return into_exit_code(e);
        }
        return 1;
    }

    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::rotate(days) {
        Ok(res) => return print_rotated(res.rotated, res.days),
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            return ewrite_or_exit(&format!("error: {e}")).map_or_else(into_exit_code, |()| 1);
        }
    }

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
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open SurrealDB: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        match kavach_surreal::rotate_events(&db, days).await {
            Ok(count) => print_rotated(count, days),
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
        }
    })
}

fn print_rotated(count: usize, days: i64) -> i32 {
    let report = format!("rotated {count} events older than {days} days");
    print_or_exit(&report).map_or_else(into_exit_code, |()| 0)
}
