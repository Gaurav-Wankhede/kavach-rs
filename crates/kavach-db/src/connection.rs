// ARCH: EmbeddedDatabase — no daemon, no server, no LOCK contention
// The DB is a single embedded libsql file owned by the process that opens it.
// Every kavach process (CLI, hook, web) opens the same file; libsql serializes
// writers via SQLite's own locking (busy_timeout=5000 set at open), so there is
// no separate single-writer LOCK to contend for at this layer.
//
// Schema is applied ONCE at open (`DatabaseStore::open` → `init_schema`), never
// per-connect — a per-connect re-apply would rebuild indexes and transiently
// empty index-backed reads.
use crate::error::Result;
use crate::db::DbStore;
use std::path::{Path, PathBuf};

/// Default database location, mirroring `DatabaseStore::open_default`
/// (`$KAVACH_HOME/kavach.db`, else `~/.kavach/kavach.db`).
#[must_use]
pub fn default_db_path() -> PathBuf {
    std::env::var_os("KAVACH_HOME").map_or_else(
        || {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".kavach")
                .join("kavach.db")
        },
        |home| PathBuf::from(home).join("kavach.db"),
    )
}

/// Open the kavach database at `path`. Schema is applied idempotently at open.
///
/// # Errors
/// Propagates `Error::Database` from open / schema bootstrap.
pub async fn open_db(path: &Path) -> Result<DbStore> {
    DbStore::open(path).await.map_err(Into::into)
}

/// Open the default database (schema applied at open).
///
/// # Errors
/// Propagates `Error::Database` from the open.
pub async fn open_default() -> Result<DbStore> {
    DbStore::open_default().await.map_err(Into::into)
}

/// Open for a long-lived holder (e.g. the RPC server's process-wide DB handle).
///
/// # Errors
/// Propagates `Error::Database` from the open.
pub async fn open_default_held() -> Result<DbStore> {
    DbStore::open_default().await.map_err(Into::into)
}

/// Open from an ephemeral hook child. Local-file open has no server-startup
/// race, so this is a plain open (kept as a distinct name for call-site intent).
///
/// # Errors
/// Propagates `Error::Database` from the open.
pub async fn open_default_resilient() -> Result<DbStore> {
    DbStore::open_default().await.map_err(Into::into)
}

/// Open an in-memory store on a fresh client (tests). No file, no server.
///
/// # Errors
/// Propagates `Error::Database` from open / schema bootstrap.
pub async fn open_memory() -> Result<DbStore> {
    DbStore::open_memory().await.map_err(Into::into)
}

#[cfg(test)]
#[path = "connection_test.rs"]
mod tests;
