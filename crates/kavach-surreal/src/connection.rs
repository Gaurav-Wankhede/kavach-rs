// ARCH: ServerClientOverWebSocket — no daemon, no embedded LOCK
// The DB is owned by a standalone `surreal start` server (launchd
// `ai.shared.kavach-surreal`, KeepAlive — restarts cleanly, never crash-loops
// like the old bespoke daemon). Every kavach process (CLI, hook, web) is a thin
// ws CLIENT of that server; the server serializes all writers, so there is no
// single-writer LOCK to contend for at this layer.
//
// PROVEN (tests/ws_decision_probe): list_by_project decodes every table over the
// ws codec (decision 321, roadmap 177) — there is NO ws decode bug. The schema
// is applied ONCE at provisioning, never per-connect (a per-connect
// `apply_schema` rebuilds the project index, which transiently empties
// index-backed reads — that was the only "bug").
// SOURCE: https://surrealdb.com/docs/surrealdb/cli/start
use crate::error::Result;
use std::path::{Path, PathBuf};
use surrealdb::Surreal;
use surrealdb::engine::any::{Any as Db, connect};
use surrealdb::opt::auth::Root;

const NS: &str = "kavach";
const MAIN_DB: &str = "main";

fn server_endpoint() -> String {
    std::env::var("KAVACH_SURREAL_ENDPOINT").unwrap_or_else(|_| "ws://127.0.0.1:7710".to_owned())
}

fn root_creds() -> (String, String) {
    (
        std::env::var("KAVACH_SURREAL_USER").unwrap_or_else(|_| "root".to_owned()),
        std::env::var("KAVACH_SURREAL_PASS").unwrap_or_else(|_| "root".to_owned()),
    )
}

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

/// Connect to the surreal server, sign in as root, select ns/db. No schema
/// apply (server persists it; per-connect apply rebuilds indexes → empty reads).
async fn connect_server(db_name: &str) -> Result<Surreal<Db>> {
    let db = connect(server_endpoint()).await?;
    let (user, pass) = root_creds();
    db.signin(Root {
        username: user,
        password: pass,
    })
    .await?;
    db.use_ns(NS).use_db(db_name).await?;
    Ok(db)
}

/// Connect with bounded backoff, tolerating a server that is still starting.
async fn connect_with_retry(db_name: &str) -> Result<Surreal<Db>> {
    let mut last = match connect_server(db_name).await {
        Ok(db) => return Ok(db),
        Err(e) => e,
    };
    for backoff_ms in [50_u64, 100, 250, 500, 1000, 1500] {
        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        match connect_server(db_name).await {
            Ok(db) => return Ok(db),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Open the kavach `main` database on the server. `path` is retained for source
/// compatibility with the former embedded API but ignored — the server owns it.
///
/// Applies schema (idempotent) because this is the provisioning entry point used
/// by the explicit `--apply-schema` path; the per-connect openers below do NOT.
///
/// # Errors
/// Propagates `Error::Surreal` from connect / signin / `use_ns` / `use_db` / schema.
pub async fn open_db(_path: &Path) -> Result<Surreal<Db>> {
    let db = connect_with_retry(MAIN_DB).await?;
    crate::schema::apply_schema(&db).await?;
    Ok(db)
}

/// Open the default `main` database (connect only — no schema apply).
///
/// # Errors
/// Propagates `Error::Surreal` from the connection.
pub async fn open_default() -> Result<Surreal<Db>> {
    connect_with_retry(MAIN_DB).await
}

/// Open for a long-lived holder (e.g. the RPC server's process-wide DB handle);
/// retry while the SurrealDB server is starting.
///
/// # Errors
/// Propagates the last `Error::Surreal` if the server is unreachable.
pub async fn open_default_held() -> Result<Surreal<Db>> {
    connect_with_retry(MAIN_DB).await
}

/// Open from an ephemeral hook child, tolerating a server mid-restart.
///
/// # Errors
/// Propagates the last `Error::Surreal` if the server stays unreachable.
pub async fn open_default_resilient() -> Result<Surreal<Db>> {
    connect_with_retry(MAIN_DB).await
}

/// Open an in-memory store on a fresh client (tests). No server needed.
///
/// # Errors
/// Propagates `Error::Surreal` from `connect` / `use_ns` / `use_db`.
pub async fn open_memory() -> Result<Surreal<Db>> {
    let db = connect("memory").await?;
    db.use_ns(NS).use_db("test").await?;
    Ok(db)
}

#[cfg(test)]
#[path = "connection_test.rs"]
mod tests;
