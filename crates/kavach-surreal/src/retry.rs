//! Operation-scoped transient-fault retry for `SurrealDB` queries.
//!
//! Connection-level recovery (`connection::open_default_held`) heals the
//! server at OPEN time. This module heals an individual query that hits a
//! TRANSIENT fault on an already-connected handle — a momentary `RocksDB`
//! busy/lock spike or a brief connection blip mid-session — which the open-time
//! backoff never covers. Self-healing must be operation-scoped, not only
//! process-scoped: a durability-critical write must survive a blip, not just the
//! server surviving.
//!
//! Classification is the safety boundary: ONLY transient faults retry. A
//! permanent error (parse error, type mismatch, missing field, record-not-found,
//! migration failure) fails FAST — retrying it would mask a real defect and
//! waste the budget. SOURCE: rocksdb#3114 (transient LOCK on concurrent access),
//! surrealdb-core 3.1.4 `err` variants; CWE-703 (proper handling of transient
//! exceptional conditions).
use crate::error::Error;
use std::future::Future;
use std::time::Duration;
/// Bounded retry budget: 5 attempts total (1 try + 4 retries), exponential
/// 25ms → 50 → 100 → 200, ~375ms worst-case added latency. Small enough to stay
/// imperceptible on the human-driven CLI path, large enough to ride out a
/// sub-second `RocksDB` contention spike. Bounded by design (CWE-835): a fault
/// that outlives the budget is NOT transient — surface it, never spin forever.
const MAX_ATTEMPTS: u32 = 5;
const BASE_BACKOFF_MS: u64 = 25;
/// True when a `SurrealDB` error is a TRANSIENT fault worth retrying.
///
/// Transient = `RocksDB` lock/busy contention, a timeout, or a
/// dropped/unavailable connection. Matched on the rendered message because the
/// SDK does not surface typed variants for these sub-errors at this version
/// (same constraint as `connection::is_lock_error`).
///
/// Conservative by construction: anything NOT positively identified as transient
/// is treated as permanent and fails fast. A false "permanent" only costs one
/// surfaced error; a false "transient" would mask a real defect AND burn budget.
#[must_use]
pub fn is_transient(err: &Error) -> bool {
    // Only raw SurrealDB errors can be transient at the engine layer; our own
    // typed variants (RecordNotFound, ProjectNotFound, Migration, InvalidHierarchy,
    // Json) are deterministic and never retryable.
    let Error::Surreal(e) = err else {
        return false;
    };
    let msg = e.to_string().to_lowercase();
    // RocksDB OS-lock contention spike on a connected handle (rocksdb#3114).
    let lock_busy = msg.contains("resource temporarily unavailable")
        || msg.contains("lock")
        || msg.contains("resource busy")
        || msg.contains("try again");
    // Momentary transport / engine-availability blip.
    let connectivity = msg.contains("timed out")
        || msg.contains("timeout")
        || msg.contains("connection")
        || msg.contains("not connected")
        || msg.contains("unavailable")
        || msg.contains("channel closed");
    lock_busy || connectivity
}
/// Run a fallible async DB `op` with bounded exponential-backoff retry on
/// TRANSIENT faults. Permanent errors return immediately on the first failure.
///
/// `op` is an async closure producing a fresh future each attempt (so the query
/// is genuinely re-issued, not a stale awaited future). The final transient
/// error is returned if the whole budget is exhausted — an honest surface, never
/// an infinite spin.
///
/// # Errors
/// Returns the operation's last error: a permanent error on first failure, or the
/// final transient error after the retry budget is exhausted.
pub async fn with_retry<T, F, Fut>(mut op: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut attempt: u32 = 0;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) if is_transient(&e) && attempt.saturating_add(1) < MAX_ATTEMPTS => {
                // Exponential backoff: 25 << attempt → 25, 50, 100, 200ms.
                // Transient retry is silent-by-design (mirrors connection.rs open
                // backoff): this crate's lint policy forbids print macros and does
                // not depend on `tracing`. The retry is transparent, and the final
                // error still surfaces honestly if the whole budget is exhausted.
                let backoff = BASE_BACKOFF_MS << attempt.min(3);
                drop(e);
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                attempt = attempt.saturating_add(1);
            }
            // Permanent error, or transient budget exhausted: surface honestly.
            Err(e) => return Err(e),
        }
    }
}
#[cfg(test)]
#[path = "retry_test.rs"]
#[cfg(test)]
#[path = "retry_test.rs"]
mod tests;
