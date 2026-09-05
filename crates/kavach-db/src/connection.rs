// ARCH: EmbeddedTurso — no daemon, no server, no LOCK contention
// The DB is a single embedded libsql file owned by the process that opens it.
// Every kavach process (CLI, hook, web) opens the same file; libsql serializes
// writers via SQLite's own locking (busy_timeout=5000 set at open), so there is
// no separate single-writer LOCK to contend for at this layer.
//
// Schema is applied ONCE at open (`TursoStore::open` → `init_schema`), never
// per-connect — a per-connect re-apply would rebuild indexes and transiently
// empty index-backed reads.
use crate::error::Result;
use crate::turso::TursoStore;
use std::path::{Path, PathBuf};

/// Default database location, mirroring `TursoStore::open_default`
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
/// Propagates `Error::Turso` from open / schema bootstrap.
pub async fn open_db(path: &Path) -> Result<TursoStore> {
    TursoStore::open(path).await.map_err(Into::into)
}

/// Open the default database (schema applied at open).
///
/// # Errors
/// Propagates `Error::Turso` from the open.
pub async fn open_default() -> Result<TursoStore> {
    TursoStore::open_default().await.map_err(Into::into)
}

/// Open for a long-lived holder (e.g. the RPC server's process-wide DB handle).
///
/// # Errors
/// Propagates `Error::Turso` from the open.
pub async fn open_default_held() -> Result<TursoStore> {
    TursoStore::open_default().await.map_err(Into::into)
}

/// Open from an ephemeral hook child. Local-file open has no server-startup
/// race, so this is a plain open (kept as a distinct name for call-site intent).
///
/// # Errors
/// Propagates `Error::Turso` from the open.
pub async fn open_default_resilient() -> Result<TursoStore> {
    TursoStore::open_default().await.map_err(Into::into)
}

/// Open an in-memory store on a fresh client (tests). No file, no server.
///
/// # Errors
/// Propagates `Error::Turso` from open / schema bootstrap.
pub async fn open_memory() -> Result<TursoStore> {
    TursoStore::open_memory().await.map_err(Into::into)
}

#[cfg(test)]
#[path = "connection_test.rs"]
mod tests;
