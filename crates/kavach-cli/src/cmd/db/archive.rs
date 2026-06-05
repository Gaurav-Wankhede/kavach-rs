use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(floor_days: i64, dry_run: bool) -> i32 {
    if floor_days <= 0 {
        if let Err(e) = ewrite_or_exit("error: floor-days must be positive") {
            return into_exit_code(e);
        }
        return 1;
    }

    // RPC-first
    match super::rpc_client::archive(floor_days, dry_run) {
        Ok(result) => {
            let mode = if result.dry_run { "DRY-RUN" } else { "APPLIED" };
            let msg = format!("[{mode}] archived={} (via rpc)", result.archived_count);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            // Daemon is UP and holds the RocksDB lock — a direct open here
            // would race it (LOCK: Resource temporarily unavailable).
            let msg = format!("rpc error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
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
        // Resilient open — closes the daemon-restart TOCTOU
        // (`rca.db-event-daemon-restart-race`): retry the lock-acquiring open
        // (bounded) instead of trusting the socket proxy; a genuine stale
        // lock still surfaces after the backoff exhausts.
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

        match kavach_surreal::archive_irrelevant(&db, floor_days, dry_run).await {
            Ok(report) => {
                let mode = if dry_run { "DRY-RUN" } else { "APPLIED" };
                let header = format!(
                    "[{mode}] scanned={} anchored={} archived={}",
                    report.scanned, report.anchored, report.archived
                );
                if let Err(io_err) = print_or_exit(&header) {
                    return into_exit_code(io_err);
                }
                if !report.samples.is_empty() {
                    let s_header = format!("samples (first {}):", report.samples.len());
                    if let Err(io_err) = print_or_exit(&s_header) {
                        return into_exit_code(io_err);
                    }
                    for s in &report.samples {
                        let line = format!("  {s}");
                        if let Err(io_err) = print_or_exit(&line) {
                            return into_exit_code(io_err);
                        }
                    }
                }
                0
            }
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
