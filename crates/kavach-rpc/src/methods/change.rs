// change.* — the GUI live-update long-poll endpoint.
//
// `change.wait(since)` blocks until the daemon's `ChangeFeed` version advances
// past `since`, then returns the new version. The GUI calls it in a loop with
// the last version it saw: the call stays parked (no CPU, no polling) until a
// real DB change lands via the LIVE SELECT watcher, then returns immediately.
// On timeout it returns the unchanged version so the GUI simply re-polls — the
// request/response socket gets event-driven semantics with bounded blocking.
// SOURCE: research.poll-vs-event-gui · state::ChangeFeed.
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Upper bound on how long a single `change.wait` parks before returning the
/// (possibly unchanged) version.
///
/// Capped UNDER the sync RPC client's 2s read timeout (client.rs sets 2s):
/// a longer park would trip that read timeout and surface as an Io error in
/// the GUI. At 1.5s a real change still returns in sub-ms (the waiter is woken
/// the instant the LIVE SELECT watcher bumps the feed) — only the *idle* case
/// costs one cheap round-trip every ~1.5s, which the GUI loop simply repeats.
const WAIT_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC request DTO; client-constructed"
)]
pub struct WaitParams {
    /// The last change version the caller has already observed.
    pub since: u64,
}

#[derive(Debug, Clone, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC response DTO; server-constructed"
)]
pub struct WaitResponse {
    /// Current change version. `> since` means data changed (re-fetch);
    /// `== since` means the wait timed out with no change (just re-poll).
    pub version: u64,
}

/// Block until the change feed advances past `since` or the timeout elapses.
///
/// # Errors
/// Never returns an error; always resolves to the current version.
pub async fn wait(state: &AppState, params: WaitParams) -> Result<WaitResponse, ErrorObjectOwned> {
    let version = state.changes.wait_past(params.since, WAIT_TIMEOUT).await;
    Ok(WaitResponse { version })
}
