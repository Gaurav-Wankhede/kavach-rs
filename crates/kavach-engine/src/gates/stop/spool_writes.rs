//! Hot-path glue between the Stop gate's fire-and-forget learning writes and the
//! durable spool primitive (`kavach_session::{enqueue_write_spool,
//! drain_write_spool}`).
//!
//! Three Stop-time RPCs (pattern seed, bandit reward, gate-pattern audit) are
//! fire-and-forget: a daemon outage must never block the Stop hook. Previously a
//! failed call was discarded, silently losing the learning signal. Now:
//!   - `call_or_spool` makes the call; on `Err` it APPENDS the write to the
//!     durable spool instead of dropping it.
//!   - `drain_and_replay`, run ONCE early on the Stop path, replays everything a
//!     prior failed Stop spooled; any replay that fails again is re-spooled.
//!
//! Every spool/replay error is best-effort (logged via `tracing`, never
//! propagated) — the gate's non-blocking contract is preserved end to end.

use kavach_session::{SpooledWrite, drain_write_spool, enqueue_write_spool};

/// Call `method` with `params`; on transport/daemon error, append it to the
/// durable spool so the next successful Stop replays it instead of losing the
/// learning signal. Never returns an error — the worst case is a best-effort
/// spool-write failure that is logged and dropped (same floor as the old
/// discard, but now only after the live call AND the durable append both fail).
pub(crate) fn call_or_spool(method: &str, params: &serde_json::Value) {
    let res: Result<serde_json::Value, _> = kavach_rpc::client::call(method, Some(params.clone()));
    if res.is_ok() {
        return;
    }
    let Ok(params_json) = serde_json::to_string(params) else {
        eprintln!("kavach spool: params not serializable for {method}; learning write lost");
        return;
    };
    let write = SpooledWrite::new(method.to_owned(), params_json);
    if let Err(e) = enqueue_write_spool(&write) {
        eprintln!("kavach spool: enqueue failed for {method} ({e}); learning write lost");
    }
}

/// Drain the durable spool and replay each write via the live RPC. A write that
/// fails again is re-spooled (via `call_or_spool`), so the signal survives across
/// many failed Stops until the daemon is back. Run ONCE, early on the Stop path,
/// BEFORE this turn's own fire-and-forget writes — drain-before-write keeps the
/// replay idempotent (the spool file is removed by `drain` before any re-append).
///
/// Best-effort: a drain read/remove error is logged and swallowed, never blocking
/// the gate.
pub(super) fn drain_and_replay() {
    let pending = match drain_write_spool() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("kavach spool: drain failed ({e}); deferring replay to next Stop");
            return;
        }
    };
    for write in pending {
        match serde_json::from_str::<serde_json::Value>(&write.params_json) {
            Ok(params) => call_or_spool(&write.method, &params),
            Err(e) => {
                eprintln!(
                    "kavach spool: replay params corrupt for {} ({e}); dropped",
                    write.method
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "spool_writes_test.rs"]
mod tests;
