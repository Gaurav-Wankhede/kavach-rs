//! Async wrapper over the kavach-rpc Unix-socket client.
//!
//! `kavach_rpc::client::call` is synchronous (blocking unix socket read), so
//! every call is moved onto a blocking thread with `spawn_blocking` to keep the
//! axum async runtime unblocked. The web server is a pure RPC consumer — it
//! never opens SurrealDB (RocksDB is single-process; the daemon owns it).

use kavach_rpc::client::{ClientError, call as raw_call};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Web-facing RPC error: either the daemon is unreachable (render a friendly
/// "start the daemon" page) or the call itself failed.
#[derive(Debug)]
pub enum RpcError {
    /// The daemon socket is absent or unreachable — the daemon is not running.
    DaemonOffline(String),
    /// Any other failure (RPC error object, I/O, decode, join).
    Failed(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DaemonOffline(p) => write!(f, "kavach-rpc daemon offline ({p})"),
            Self::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl RpcError {
    /// True when the failure is "daemon not running" (drives the offline page).
    #[must_use]
    pub const fn is_offline(&self) -> bool {
        matches!(self, Self::DaemonOffline(_))
    }
}

fn map_err(e: ClientError) -> RpcError {
    match e {
        ClientError::NotReachable(p) => RpcError::DaemonOffline(p),
        ClientError::Rpc { code, message } => RpcError::Failed(format!("rpc {code}: {message}")),
        ClientError::Io(e) => RpcError::Failed(format!("io: {e}")),
        ClientError::Json(e) => RpcError::Failed(format!("decode: {e}")),
        ClientError::NoResult => RpcError::Failed("missing result in response".to_owned()),
    }
}

/// Call an RPC method with params, off the async runtime.
///
/// # Errors
/// Returns [`RpcError::DaemonOffline`] when the daemon is unreachable, else
/// [`RpcError::Failed`] for RPC/IO/decode/join failures.
pub async fn call<P, R>(method: &str, params: P) -> Result<R, RpcError>
where
    P: Serialize + Send + 'static,
    R: DeserializeOwned + Send + 'static,
{
    let method = method.to_owned();
    tokio::task::spawn_blocking(move || raw_call::<P, R>(&method, Some(params)).map_err(map_err))
        .await
        .map_err(|e| RpcError::Failed(format!("join: {e}")))?
}

/// Call a no-params RPC method, off the async runtime.
///
/// # Errors
/// Same as [`call`].
pub async fn call_no_params<R>(method: &str) -> Result<R, RpcError>
where
    R: DeserializeOwned + Send + 'static,
{
    let method = method.to_owned();
    tokio::task::spawn_blocking(move || raw_call::<(), R>(&method, None).map_err(map_err))
        .await
        .map_err(|e| RpcError::Failed(format!("join: {e}")))?
}
