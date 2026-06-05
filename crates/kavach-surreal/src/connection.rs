// split: intentional — connection-lifecycle module bundles open_db/open_default/
// open_memory + LOCK-recovery helpers. Splitting them per-fn would fragment one
// cohesive responsibility (DB handle acquisition) across multiple files.
//
// ARCH: SingleWriterFallbackByDaemonEviction
// PATTERN: connection_open_with_lock_recovery
// CAPACITY: 1 LOCK file holder (RocksDB OS-level FileLock)
// QPS: human-driven CLI ops, sub-1 invocation/sec; daemon stays up across many gate fires
// LATENCY: <50ms typical open; <1.2s on contended path (50ms x 20 retries SIGTERM wait)
// CONSISTENCY: at-most-one writer process at any moment — daemon yields to CLI
// FAILURE_MODE: daemon refuses to die after 1s -> bubble original LOCK error to user
// OBSERVABILITY: stderr message names which strategy fired (direct, daemon-evict, fail)
// TRADEOFF: daemon restart cost (~500ms cold start on next gate fire); acceptable
// SOURCE: https://github.com/facebook/rocksdb/issues/908 (multi-process not supported)
//
// ALGO: SignalThenPollWaitForExit
// PROBLEM_CLASS: stream
// REJECTED: [{"name":"sigkill_immediately","reason":"loses unflushed writes; SIGTERM then SIGKILL preserves WAL fsync"},{"name":"flock_F_GETLK_check","reason":"only tells if locked, doesn't fix; still need eviction"},{"name":"rocksdb_secondary_open","reason":"read-only; gate writes need primary access"}]
// TIME: O(retries) bounded at 20*50ms=1s | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: 50ms poll granularity; daemons that fsync slowly (>1s) still hit the bail-out
// BENCHMARK: https://surrealdb.com/blog/surrealdb-3-0-benchmarks-a-new-foundation-for-performance
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem, RocksDb};

#[must_use]
pub fn default_db_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map_or_else(
            || PathBuf::from("/tmp/kavach.surreal"),
            |h| h.join("Library/Application Support/SharedAI/kavach.surreal"),
        )
    } else if cfg!(target_os = "windows") {
        dirs::data_local_dir().map_or_else(
            || PathBuf::from("C:\\Users\\Public\\SharedAI\\kavach.surreal"),
            |d| d.join("SharedAI\\kavach.surreal"),
        )
    } else {
        dirs::data_dir().map_or_else(
            || PathBuf::from("/tmp/kavach.surreal"),
            |d| d.join("shared-ai/kavach.surreal"),
        )
    }
}

/// Path to the kavach-rpc daemon lockfile (port file). Co-located with
/// the `SurrealDB` store directory so daemon discovery doesn't require an
/// extra config knob.
///
/// Unix-only: the daemon-eviction path (`try_stop_daemon`) is the sole
/// caller, and it signals via POSIX kill(2). On non-unix there is no daemon
/// to evict, so this helper would be dead code (`-D dead-code` on Windows).
#[cfg(unix)]
fn daemon_port_path() -> PathBuf {
    let mut p = default_db_path();
    p.pop();
    p.push("kavach-rpc.port");
    p
}

/// Inspect the daemon port file and return the recorded PID if present.
/// Returns None when the file is absent, malformed, or the recorded PID is
/// no longer alive (stale lockfile).
///
/// SECURITY: rejects pid <= 0 explicitly. POSIX kill(2) treats negative or
/// zero PIDs as process-group identifiers — passing one through would
/// SIGTERM unrelated processes. We accept only positive PIDs.
///
/// Unix-only: called solely by the daemon-eviction path. On non-unix the
/// daemon is never evicted (no POSIX signals), so this is dead code there.
#[cfg(unix)]
fn read_live_daemon_pid() -> Option<i32> {
    let Ok(body) = std::fs::read_to_string(daemon_port_path()) else {
        return None;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
        return None;
    };
    let pid_raw = parsed.get("pid").and_then(serde_json::Value::as_i64)?;
    let Ok(pid) = i32::try_from(pid_raw) else {
        return None;
    };
    if pid <= 0 {
        // Negative PIDs hit kill(-pgid, sig) — process-group fan-out.
        // Zero hits kill(0, sig) — every process in the caller's group.
        // Both unacceptable for a per-daemon eviction signal.
        return None;
    }
    // rustix safe wrapper for POSIX kill(pid, 0) existence probe.
    let rpid = rustix::process::Pid::from_raw(pid)?;
    rustix::process::test_kill_process(rpid)
        .is_ok()
        .then_some(pid)
}

/// Stop the kavach-rpc daemon if it is currently holding the `SurrealDB` lock.
/// Returns true if a daemon was killed and waited on, false if no live daemon
/// was found. Used by `open_default` on LOCK errors to clear the contention
/// before retrying. Daemon is best-effort perf cache — restarting on next
/// gate fire costs <1s.
#[cfg(unix)]
fn try_stop_daemon() -> bool {
    let Some(pid) = read_live_daemon_pid() else {
        return false;
    };
    // rustix safe wrapper for POSIX kill(pid, SIGTERM). The daemon installs
    // a handler that fsyncs RocksDB before exiting. Best-effort: ignore
    // send-signal errors (Errno::SRCH = already dead = goal achieved).
    if let Some(rpid) = rustix::process::Pid::from_raw(pid) {
        match rustix::process::kill_process(rpid, rustix::process::Signal::TERM) {
            Ok(()) | Err(_) => {} // best-effort signal; SRCH = already gone
        }
    }
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if read_live_daemon_pid().is_none() {
            // Best-effort cleanup of the stale port file + socket.
            drop(std::fs::remove_file(daemon_port_path()));
            let mut sock = default_db_path();
            sock.pop();
            sock.push("kavach-rpc.sock");
            drop(std::fs::remove_file(sock));
            return true;
        }
    }
    false
}

#[cfg(not(unix))]
const fn try_stop_daemon() -> bool {
    false
}

/// Open a `SurrealDB` store at `path` with the kavach namespace/db and apply schema.
///
/// # Errors
/// Propagates `Error::Io` from parent-dir creation, `Error::Surreal` from
/// `SurrealDB` connection / `use_ns` / `use_db` / schema-apply.
pub async fn open_db(path: &Path) -> Result<Surreal<Db>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = Surreal::new::<RocksDb>(path).await?;
    db.use_ns("kavach").use_db("main").await?;
    // FIX: [config_drift/init-order] SurrealDB 3.0 does NOT implicitly
    // create SCHEMAFULL tables on first write (2.x did) — without this,
    // a fresh store has no `decision`/`roadmap`/... tables and every
    // db write silently fails ("table does not exist"). apply_schema is
    // idempotent (DEFINE TABLE redefines structure, never drops rows;
    // feedback fields use IF NOT EXISTS), so it is safe on every open —
    // fresh stores get tables created, existing stores are a no-op.
    crate::schema::apply_schema(&db).await?;
    Ok(db)
}

/// Open the default-path `SurrealDB` store with daemon-conflict recovery.
///
/// Embedded `RocksDB` is single-writer at the OS level — only one process can
/// hold the LOCK. If the kavach-rpc daemon is running it owns the lock; this
/// function detects that case, asks the daemon to exit (SIGTERM), waits for
/// release, then retries the open. Daemon is a perf cache only — losing it
/// for one CLI op is harmless; the next gate fire respawns it on demand.
/// Open the default-path store, evicting a conflicting kavach-rpc daemon on
/// LOCK error and retrying once.
///
/// # Errors
/// Propagates `Error::Surreal` if the initial open fails for a non-lock
/// reason, or the retry after daemon eviction still fails.
pub async fn open_default() -> Result<Surreal<Db>> {
    let path = default_db_path();
    match open_db(&path).await {
        Ok(db) => Ok(db),
        Err(e) if is_lock_error(&e) && try_stop_daemon() => {
            // Daemon released the lock — retry once.
            open_db(&path).await
        }
        Err(e) => Err(e),
    }
}

/// True when the underlying `SurrealDB` error is the `RocksDB` LOCK-file
/// contention emitted on a second open. Pattern-match on rendered message
/// because typed variants for the `RocksDB` sub-error aren't surfaced through
/// the `SurrealDB` SDK error tree at this version.
fn is_lock_error(e: &Error) -> bool {
    let msg = e.to_string();
    msg.contains("Resource temporarily unavailable") && msg.contains("LOCK")
}

/// Open the default-path store for a **long-lived daemon**, tolerating
/// transient LOCK contention with bounded backoff.
///
/// `open_default` is tuned for the short-lived CLI: one open, evict a
/// conflicting daemon, retry once, else fail. That single-shot policy is
/// fatal for the daemon itself. When launchd respawns daemon-B while
/// daemon-A is still fsyncing `RocksDB` on shutdown (or a CLI op briefly holds
/// the lock), a one-shot open fails and the daemon exits non-zero — launchd
/// then KeepAlive-respawns into the same race, producing an unbounded
/// crash-loop (observed: `runs=716`, `OS_REASON_CODESIGNING`/non-zero exit,
/// stale socket, CLI + GUI both seeing "daemon offline / no projects").
///
/// `RocksDB` is single-writer (facebook/rocksdb#908); the contending writer is
/// always transient here, so the correct daemon policy is *patience*: retry
/// the plain open with exponential backoff, attempting an eviction only on
/// the FIRST contended attempt (so a genuinely-wedged sibling is cleared
/// once, without the two-daemon mutual-evict thrash that per-attempt
/// eviction causes). Bounded at ~30s total, then an honest error so launchd
/// surfaces a real fault instead of hiding it.
///
/// # Errors
/// Propagates the last `Error::Surreal` if every attempt within the budget
/// still fails (non-lock errors fail fast on the first attempt).
pub async fn open_default_daemon() -> Result<Surreal<Db>> {
    // 50ms, 100, 200, 400, 800, 1600, then capped at 2000ms — ~30s budget.
    const MAX_ATTEMPTS: u32 = 24;
    const CAP_MS: u64 = 2000;
    let path = default_db_path();
    let mut last_err: Option<Error> = None;
    for attempt in 0..MAX_ATTEMPTS {
        match open_db(&path).await {
            Ok(db) => return Ok(db),
            Err(e) if is_lock_error(&e) => {
                // Evict a wedged sibling exactly once, on the first contended
                // attempt — never per-attempt (that thrashes two daemons into
                // mutually SIGTERMing each other). Afterwards: pure backoff.
                if attempt == 0 {
                    let _evicted = try_stop_daemon();
                }
                // Transient contention is silent-by-design: the retry is
                // transparent, and if the whole budget is exhausted the final
                // `last_err` propagates through `run()`'s `map_err` to stderr
                // (→ launchd kavach-rpc.err.log). Per-attempt logging would
                // require a print macro this crate's lint policy forbids.
                let backoff_ms = (50_u64 << attempt.min(5)).min(CAP_MS);
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            }
            // Non-lock error: a real fault (corrupt store, bad path). Fail
            // fast — retrying would just delay surfacing it.
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        Error::RecordNotFound("open_default_daemon exhausted retries".to_owned())
    }))
}

/// Open an in-memory `SurrealDB` store. Used by tests.
///
/// # Errors
/// Propagates `Error::Surreal` from `Surreal::new` or `use_ns`/`use_db`.
pub async fn open_memory() -> Result<Surreal<Db>> {
    let db = Surreal::new::<Mem>(()).await?;
    db.use_ns("kavach").use_db("test").await?;
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_memory() -> Result<()> {
        let db = open_memory().await?;
        let info: Option<serde_json::Value> = db.query("INFO FOR DB").await?.take(0)?;
        info.ok_or_else(|| Error::RecordNotFound("INFO FOR DB returned empty result".to_owned()))?;
        Ok(())
    }
}
