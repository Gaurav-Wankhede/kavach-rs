//! kavach-rpc — JSON-RPC 2.0 adapter for kavach-surreal.
//!
//! INVARIANT: stdout is the JSON-RPC protocol channel in stdio mode.
//! All logging MUST go to stderr (caller responsibility via tracing-subscriber).
pub mod client;
pub mod error;
pub mod lease_janitor;
pub mod live_watch;
pub mod lockfile;
pub mod methods;
pub mod rpc;
pub mod state;
pub mod transport;

use kavach_surreal::{apply_agent_memory_schema, apply_schema, open_default_daemon};
use state::AppState;

#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::exhaustive_enums,
    reason = "constructed at runtime with fixed variants"
)]
pub enum TransportKind {
    Stdio,
    Http,
    #[cfg(unix)]
    Unix,
}

/// Run the kavach-rpc server.
///
/// Opens an embedded `SurrealDB` connection, optionally applies schema,
/// builds the `RpcModule`, and dispatches to the chosen transport.
/// Blocks until the transport exits (EOF on stdio, ctrl-c on http).
///
/// # Errors
///
/// Returns an error if `SurrealDB` connection fails, schema application fails, RPC module
/// construction fails, or the selected transport fails to run.
pub async fn run(transport: TransportKind, apply_schema_on_start: bool) -> Result<(), String> {
    // Daemon open: tolerate transient RocksDB LOCK contention with bounded
    // backoff instead of the CLI's evict-once-or-die policy. Without this a
    // launchd-respawned daemon racing a dying sibling exits non-zero and
    // KeepAlive crash-loops (observed runs=716), leaving a stale socket so
    // CLI + GUI both see "daemon offline / no projects".
    let db = open_default_daemon()
        .await
        .map_err(|e| format!("open SurrealDB: {e}"))?;

    if apply_schema_on_start {
        apply_schema(&db)
            .await
            .map_err(|e| format!("apply schema: {e}"))?;
        apply_agent_memory_schema(&db)
            .await
            .map_err(|e| format!("apply agent memory schema: {e}"))?;
    }

    let state = AppState::new(db);
    live_watch::spawn(
        std::sync::Arc::clone(&state.db),
        std::sync::Arc::clone(&state.changes),
    );
    // Renew held leases on a TTL/3 cadence so a session working a card longer
    // than the lease TTL keeps its claim (crashed holders still lapse).
    lease_janitor::spawn(std::sync::Arc::clone(&state.db));
    let module = rpc::build_module(state).map_err(|e| format!("build module: {e}"))?;

    match transport {
        TransportKind::Stdio => transport::stdio::run(module)
            .await
            .map_err(|e| format!("stdio: {e}")),
        TransportKind::Http => transport::http::run(module)
            .await
            .map_err(|e| format!("http: {e}")),
        #[cfg(unix)]
        TransportKind::Unix => transport::unix::run(module)
            .await
            .map_err(|e| format!("unix: {e}")),
    }
}
