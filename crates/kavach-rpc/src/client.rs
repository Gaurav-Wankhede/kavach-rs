// In-process RPC dispatch — NO daemon, NO Unix socket.
//
// kavach-engine gates run synchronously inside Claude Code hooks. Previously this
// client did a sync Unix-socket round-trip to a long-running daemon. The daemon
// is retired: the DB now lives in a `surreal start` server, and every kavach
// process is a thin ws client. So `call` opens a ws connection to the server,
// builds the SAME `RpcModule` the server used, and dispatches the request
// in-process via `raw_json_request` — byte-identical to the old daemon transport
// (see transport/unix.rs), just without the socket hop. The 54 call sites and
// every registered method are untouched.
//
// The runtime + module are built once per process (OnceLock) and reused, so a
// hook pays one ws connect, not one per call.
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively cross-crate; non_exhaustive => E0004"
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
    #[error("surreal server not reachable: {0}")]
    NotReachable(String),
}

/// Process-global in-process dispatcher: a current-thread runtime + the
/// ws-connected `RpcModule`. Built once on first `call`.
struct InProc {
    rt: tokio::runtime::Runtime,
    module: jsonrpsee::RpcModule<crate::state::AppState>,
}

fn build_inproc() -> Result<InProc, String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let db = rt
        .block_on(kavach_surreal::open_default_held())
        .map_err(|e| format!("connect surreal server: {e}"))?;
    let state = crate::state::AppState::new(db);
    let module = crate::rpc::build_module(state).map_err(|e| format!("build module: {e}"))?;
    Ok(InProc { rt, module })
}

fn inproc() -> Result<&'static InProc, ClientError> {
    static CELL: OnceLock<Result<InProc, String>> = OnceLock::new();
    // `build_inproc` does `rt.block_on(...)` to open the DB. If the FIRST
    // caller is already inside a tokio runtime (a CLI command in its own
    // `block_on`, a gate hook on an async thread), running that init on the
    // current thread panics with "Cannot start a runtime from within a
    // runtime". Build on a dedicated thread so the init `block_on` is never
    // nested. The result is cached in the OnceLock either way, so the
    // thread-spawn is paid at most ONCE per process.
    let built = CELL.get_or_init(|| {
        if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::scope(|s| s.spawn(build_inproc).join())
                .unwrap_or_else(|_| Err("inproc init worker panicked".to_owned()))
        } else {
            build_inproc()
        }
    });
    match built {
        Ok(ip) => Ok(ip),
        Err(e) => Err(ClientError::NotReachable(e.clone())),
    }
}

/// Dispatch a JSON-RPC method in-process against the surreal-server-backed
/// module. Drop-in replacement for the former Unix-socket call.
///
/// # Errors
/// `NotReachable` if the server connection / module build failed; `Json` on
/// (de)serialization failure; `Rpc` if the method returned a JSON-RPC error;
/// `NoResult` if a success response carried no `result` key.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Option<P> by value is the ergonomic call signature across 100+ call \
              sites; the json! macro serializes it by reference, but switching to \
              Option<&P> would ripple a borrow through every caller for no win"
)]
pub fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
    method: &str,
    params: Option<P>,
) -> Result<R, ClientError> {
    let ip = inproc()?;
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });
    let request_str = serde_json::to_string(&request)?;

    // `call` is SYNC but the dispatch is async on the InProc runtime. If the
    // CALLER is already inside a tokio runtime (e.g. a CLI command that built
    // its own `Runtime` and is in `block_on`, or a gate hook on an async
    // thread), calling `ip.rt.block_on` on THIS thread panics with "Cannot
    // start a runtime from within a runtime". Detect the ambient runtime and,
    // when present, run the blocking dispatch on a SEPARATE thread so the
    // current thread is never blocked-on twice. Off-runtime callers keep the
    // direct `block_on` (no thread-spawn cost). The InProc runtime is
    // multi-thread and independent of the caller's, so the helper thread's
    // `block_on` is safe.
    let dispatch = || {
        ip.rt
            .block_on(async { ip.module.raw_json_request(&request_str, 1).await })
    };
    let raw_result = if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|s| s.spawn(dispatch).join())
            .map_err(|_| ClientError::NotReachable(format!("dispatch {method}: worker panicked")))?
    } else {
        dispatch()
    };
    let (response, _stream) =
        raw_result.map_err(|e| ClientError::NotReachable(format!("dispatch {method}: {e}")))?;

    // Parse the JSON-RPC response: distinguish present-but-null `result` (valid
    // empty answer) from an absent `result` key (protocol error). Mirrors the
    // prior client's null-vs-absent fix.
    let raw: serde_json::Value = serde_json::from_str(&response)?;
    if let Some(err_obj) = raw.get("error").filter(|e| !e.is_null()) {
        let code = err_obj
            .get("code")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(-1);
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
