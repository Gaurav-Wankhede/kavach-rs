// split: intentional - sync JSON-RPC client for kavach-engine gate hot path
// Per research.engine-async-blocker (id=3759): kavach-engine gates run sync inside
// Claude Code hooks. This client wraps std::os::unix::net::UnixStream (sync) so
// gates can call kavach-rpc without spawning a tokio runtime per invocation.
// One persistent kavach-rpc daemon serves all gates over UDS.
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(unix)]
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively cross-crate in kavach-app rpc_client.rs; non_exhaustive => E0004"
)]
pub enum ClientError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("rpc error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("missing result in response")]
    NoResult,
    #[error("kavach-rpc daemon not reachable at {0}")]
    NotReachable(String),
}

// Unix-only: constructed solely by the #[cfg(unix)] `call` over UDS. The
// non-unix `call` stub returns NotReachable without building a request, so
// this would be dead code on Windows (`-D dead-code`).
#[cfg(unix)]
#[derive(Serialize)]
struct JsonRpcRequest<'a, P> {
    jsonrpc: &'static str,
    method: &'a str,
    params: Option<P>,
    id: u64,
}

// Response is decoded as a raw serde_json::Value so the client can
// distinguish a present-but-null `result` (valid empty answer) from an
// absent `result` (malformed response) — see the FIX block in `call`.
// Typed JsonRpcResponse/RpcErrorPayload structs were removed: Option<T>'s
// Deserialize collapses JSON null into None, which is the exact bug.

#[cfg(unix)]
static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(unix)]
fn default_socket_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map_or_else(
            || PathBuf::from("/tmp/kavach-rpc.sock"),
            |h| h.join("Library/Application Support/SharedAI/kavach-rpc.sock"),
        )
    } else {
        dirs::data_dir().map_or_else(
            || PathBuf::from("/tmp/kavach-rpc.sock"),
            |d| d.join("shared-ai/kavach-rpc.sock"),
        )
    }
}

/// Synchronous Unix-socket JSON-RPC call. One-shot connect-write-read-close.
///
/// Designed for kavach-engine gates that run inside Claude Code hooks.
/// Returns Err if the daemon is not running (caller should fall back to direct `SurrealDB`).
///
/// # Errors
///
/// Returns `ClientError::NotReachable` if the daemon socket does not exist or connection fails
/// with transient errors (`ConnectionRefused`, `NotFound`, `ConnectionReset`).
/// Returns `ClientError::Io` for other I/O failures (timeout, permission denied, etc.).
/// Returns `ClientError::Json` if request serialization or response deserialization fails.
/// Returns `ClientError::Rpc` if the daemon returns a JSON-RPC error object.
/// Returns `ClientError::NoResult` if the response lacks the required `result` field.
#[cfg(unix)]
pub fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
    method: &str,
    params: Option<P>,
) -> Result<R, ClientError> {
    use std::io::ErrorKind::{ConnectionRefused, ConnectionReset, NotFound};

    let dbg = std::env::var("KAVACH_RPC_CLIENT_DEBUG").is_ok();
    let socket_path = default_socket_path();
    if dbg {
        tracing::warn!(
            method,
            socket = %socket_path.display(),
            exists = socket_path.exists(),
            "[rpc-client] attempt"
        );
    }
    if !socket_path.exists() {
        return Err(ClientError::NotReachable(socket_path.display().to_string()));
    }

    let stream = match UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(e) => {
            if dbg {
                tracing::warn!(error = %e, "[rpc-client] connect failed");
            }
            // A stale socket can pass the exists() pre-check while the daemon
            // is down; these three kinds mean "transiently unreachable" and
            // must map to NotReachable (so the caller falls back), not Io.
            // rca.db-event-daemon-restart-race. `matches!` (not match+`_`):
            // io::ErrorKind is #[non_exhaustive], so any wildcard arm is the
            // RUST_GUARD-flagged catch-all — a boolean predicate enumerates
            // the handled set explicitly and leaves no hidden arm.
            let transient_restart =
                matches!(e.kind(), ConnectionRefused | NotFound | ConnectionReset);
            if transient_restart {
                return Err(ClientError::NotReachable(socket_path.display().to_string()));
            }
            return Err(e.into());
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let id = REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        method,
        params,
        id,
    };
    let req_bytes = serde_json::to_vec(&req)?;

    let mut writer = stream;
    writer.write_all(&req_bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()?;

    let mut reader = BufReader::new(writer);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let trimmed = line.trim();
    if dbg {
        tracing::warn!(response = trimmed, "[rpc-client] raw response");
    }

    // FIX: [API Contract / null-vs-absent] [client.rs:133]
    // SYMPTOM: every stop-gate eval got Err(NoResult) -> SOURCE_DOWN -> loop
    //   blocked forever, even with a healthy daemon and an empty kanban.
    // WHY5: JSON-RPC 2.0 §5 — a success response MUST carry `result`, and
    //   `null` is a valid result value. serde's Option<T> Deserialize maps
    //   JSON `null` to None (serde-rs/json #376, serde-rs/serde #984), so
    //   `resp.result.ok_or(NoResult)?` turned the CORRECT "no open task"
    //   answer (`{"result":null}`) into a client error.
    // ROOT_CAUSE: client conflated "result key absent" (protocol error) with
    //   "result value is null" (valid empty answer).
    // BLAST_SITE: 1 of 1 (sole kavach_rpc::client::call decode path).
    // RESEARCH: https://www.jsonrpc.org/specification §5 ;
    //   https://github.com/serde-rs/json/issues/376 (null vs absent).
    // SOLUTION: inspect the raw JSON for `result` KEY presence — present
    //   (incl. explicit null) -> deserialize into R; absent (and no error)
    //   -> NoResult. Equivalent to the serde_with double_option pattern
    //   without adding a dependency for one call site.
    let raw: serde_json::Value = serde_json::from_str(trimmed)?;
    if let Some(err_obj) = raw.get("error").filter(|e| !e.is_null()) {
        let code = err_obj
            .get("code")
            .and_then(serde_json::Value::as_i64)
            .map_or(-1, |c| c);
        let message = err_obj
            .get("message")
            .and_then(|m| m.as_str())
            .map_or_else(|| "unknown rpc error".to_owned(), str::to_owned);
        return Err(ClientError::Rpc { code, message });
    }
    match raw.get("result") {
        Some(result) => Ok(serde_json::from_value(result.clone())?),
        None => Err(ClientError::NoResult),
    }
}

/// Non-unix fallback: the synchronous UDS transport is unix-only, so this
/// always reports the daemon as unreachable and the caller falls back to a
/// direct `SurrealDB` open.
///
/// # Errors
///
/// Always returns `ClientError::NotReachable` — there is no Unix-socket
/// transport on this platform.
#[cfg(not(unix))]
pub fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
    _method: &str,
    _params: Option<P>,
) -> Result<R, ClientError> {
    Err(ClientError::NotReachable(
        "Unix socket transport not available on this platform".into(),
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::ErrorKind;

    /// Mirrors the connect-error classification in `call`: a stale socket can
    /// pass `exists()` while the daemon restarts, so these kinds must be
    /// treated as transiently-unreachable (→ `NotReachable` → caller falls
    /// back), not as a hard `Io` fault (→ "rpc error", no fallback — the
    /// residual `rca.db-event-daemon-restart-race` live-prove caught).
    const fn is_transient_restart(k: ErrorKind) -> bool {
        matches!(
            k,
            ErrorKind::ConnectionRefused | ErrorKind::NotFound | ErrorKind::ConnectionReset
        )
    }

    #[test]
    fn connect_restart_kinds_are_transient() {
        for k in [
            ErrorKind::ConnectionRefused, // old daemon dead, nothing accepting
            ErrorKind::NotFound,          // socket unlinked mid-restart
            ErrorKind::ConnectionReset,   // daemon died mid-accept
        ] {
            assert!(is_transient_restart(k), "{k:?} must route to NotReachable");
        }
    }

    #[test]
    fn genuine_io_faults_are_not_transient() {
        // Real faults must stay Io — must NOT spuriously trigger a fallback
        // (which would then bounded-retry a permanent error).
        for k in [
            ErrorKind::PermissionDenied,
            ErrorKind::InvalidInput,
            ErrorKind::Other,
        ] {
            assert!(
                !is_transient_restart(k),
                "{k:?} must stay Io, not fall back"
            );
        }
    }
}
