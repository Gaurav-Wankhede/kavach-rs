//! Durable spool for fire-and-forget RPC writes that must NOT be lost on a DB blip.
//!
//! The learning-loop writes (pattern seed, bandit reward, gate audit) are emitted
//! from the Stop gate, which must never block. Previously a failed RPC was
//! discarded (`let _: Result = call(...)`), silently losing the signal the whole
//! self-learning loop depends on. This spool is the non-blocking, non-lossy
//! middle ground: a failed write is APPENDED as one JSON line to a durable file
//! under `state_dir()`; the next successful Stop drains and replays it.
//!
//! Format: line-delimited JSON (one `SpooledWrite` per line). An append of a
//! single newline-terminated line is atomic on every mainstream filesystem, so a
//! crash mid-append never corrupts an earlier line. SOURCE:
//! <https://users.rust-lang.org/t/correct-way-to-save-a-file-atomically-but-without-interferring-with-performance/89223>

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::state_dir;

/// One deferred RPC write: the method name + its JSON params, enough to replay
/// `kavach_rpc::client::call(method, params)` verbatim on drain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpooledWrite {
    /// The RPC method to replay (e.g. `db.write`, `db.bandit_backfill_session`).
    pub method: String,
    /// The JSON params object passed to the original call, serialized as a string
    /// so the spool stays a flat line-delimited file with no nested escaping rules.
    pub params_json: String,
}

impl SpooledWrite {
    /// Construct a spooled write from a method name and its serialized params.
    /// Provided because the struct is `#[non_exhaustive]` (cross-crate callers
    /// like the engine glue cannot use a struct literal).
    #[must_use]
    pub const fn new(method: String, params_json: String) -> Self {
        Self {
            method,
            params_json,
        }
    }
}

/// Absolute path to the spool file under the session state dir.
fn spool_path() -> PathBuf {
    state_dir().join("write_spool.jsonl")
}

/// Append one deferred write to the durable spool (one JSON line, flushed).
///
/// Non-blocking by construction: a single newline-terminated line append is
/// atomic, so concurrent Stop hooks never interleave a partial line.
///
/// # Errors
/// Returns `Err` if the state dir cannot be created or the append/flush fails —
/// the caller treats an enqueue failure as "best-effort lost" (same floor as the
/// old discard), never a blocked Stop.
pub fn enqueue(write: &SpooledWrite) -> io::Result<()> {
    let path = spool_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(write).map_err(io::Error::other)?;
    let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

/// Read every spooled write and REMOVE the file, returning entries in append order.
///
/// The caller replays each via `kavach_rpc::client::call`; any that
/// fail again are re-`enqueue`d. Removing the file before replay is intentional:
/// a replay that crashes mid-drain re-enqueues only the survivors, never
/// double-replays a write that already landed.
///
/// A missing spool (the common case — nothing failed) returns an empty Vec.
/// Unparseable lines are skipped (a corrupt tail never strands the whole spool).
///
/// # Errors
/// Returns `Err` only if the file exists but cannot be read or removed.
pub fn drain() -> io::Result<Vec<SpooledWrite>> {
    let path = spool_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    fs::remove_file(&path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<SpooledWrite>(l).ok())
        .collect())
}

#[cfg(test)]
#[path = "write_spool_test.rs"]
#[cfg(test)]
#[path = "write_spool_test.rs"]
mod tests;