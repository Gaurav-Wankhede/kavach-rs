use kavach_rpc::client::{ClientError, call as raw_call};
use serde::{Serialize, de::DeserializeOwned};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    DaemonOffline(String),
    Rpc { code: i64, message: String },
    Io(String),
    Decode(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DaemonOffline(s) => write!(f, "kavach-rpc daemon offline ({s})"),
            Self::Rpc { code, message } => write!(f, "rpc error {code}: {message}"),
            Self::Io(s) => write!(f, "io error: {s}"),
            Self::Decode(s) => write!(f, "decode error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ClientError> for Error {
    fn from(e: ClientError) -> Self {
        match e {
            ClientError::NotReachable(p) => Self::DaemonOffline(p),
            ClientError::Rpc { code, message } => Self::Rpc { code, message },
            ClientError::Io(e) => Self::Io(e.to_string()),
            ClientError::Json(e) => Self::Decode(e.to_string()),
            ClientError::NoResult => Self::Decode("missing result in response".into()),
        }
    }
}

impl Error {
    #[must_use]
    pub const fn is_daemon_offline(&self) -> bool {
        matches!(self, Self::DaemonOffline(_))
    }
}

pub fn rpc<P, R>(method: &str, params: P) -> Result<R, Error>
where
    P: Serialize,
    R: DeserializeOwned,
{
    raw_call::<P, R>(method, Some(params)).map_err(Into::into)
}

pub fn rpc_no_params<R>(method: &str) -> Result<R, Error>
where
    R: DeserializeOwned,
{
    raw_call::<(), R>(method, None).map_err(Into::into)
}

#[derive(Serialize)]
struct WaitParams {
    since: u64,
}

#[derive(serde::Deserialize)]
struct WaitResponse {
    version: u64,
}

/// Long-poll the daemon's change feed: blocks (server-side, bounded) until the
/// change version advances past `since`, then returns the new version. A real
/// DB change returns near-instantly; an idle period returns the unchanged
/// `since` after the daemon's short park window so the caller simply re-polls.
/// This is the event-driven engine behind the GUI's live refresh — no client
/// polling timer, the daemon pushes via `SurrealDB` LIVE SELECT.
///
/// # Errors
/// Propagates daemon-offline / RPC / decode errors so the caller can back off.
pub fn wait_for_change(since: u64) -> Result<u64, Error> {
    let resp: WaitResponse = rpc("change.wait", WaitParams { since })?;
    Ok(resp.version)
}
