//! Witness receipt: the cheap, non-blocking proof token a `roadmap` completion
//! promotion must carry across the RPC boundary. The CLI/agent runs the heavy
//! workspace witness (cargo check+clippy+nextest), then mints a receipt; the
//! daemon VALIDATES it in O(1) (one `git rev-parse HEAD`) without ever spawning
//! cargo — so the same evidence contract the CLI enforces also binds every direct
//! RPC caller, with no tokio-worker block and no nextest→hook→daemon re-entrancy.
//! SOURCE: decision.cli-verifier.witness-receipt-rpc-boundary.

use serde::{Deserialize, Serialize};

/// Freshness window: a receipt older than this is stale (the tree may have moved).
const FRESHNESS_MS: i64 = 300_000; // 5 minutes

/// A proof that the workspace witnesses passed against a specific commit, for a
/// specific session, at a specific time. Minted only by the witness path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Receipt {
    /// Whether all witnesses (check+clippy+nextest+diff) passed.
    pub passed: bool,
    /// `git rev-parse HEAD` at the moment the witness ran — anti-replay anchor.
    pub git_head: String,
    /// Epoch milliseconds the witness completed — freshness anchor.
    pub ts_ms: i64,
    /// The session that minted it — anti cross-session replay.
    pub session_id: String,
}

impl Receipt {
    /// Construct a receipt (non-exhaustive struct needs a cross-crate constructor).
    #[must_use]
    pub const fn new(passed: bool, git_head: String, ts_ms: i64, session_id: String) -> Self {
        Self { passed, git_head, ts_ms, session_id }
    }
}

/// Why a receipt was refused — surfaced verbatim to the agent so it knows to
/// re-run the witness rather than guess.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiptError {
    /// The witness itself reported failure.
    WitnessFailed,
    /// The tree moved since the witness ran (`git_head != HEAD`).
    HeadMismatch,
    /// Older than the freshness window, or future-dated (clock skew/forgery).
    Stale,
    /// No session bound, or it does not match the promoting session.
    SessionMismatch,
}

impl std::fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::WitnessFailed => "witness reported FAILED",
            Self::HeadMismatch => "git HEAD moved since the witness ran",
            Self::Stale => "receipt is stale or future-dated",
            Self::SessionMismatch => "receipt session does not match promoting session",
        };
        f.write_str(s)
    }
}

/// Validate a receipt against the daemon's live view. Cheap and total — no I/O.
///
/// The caller supplies `head_now`/`now_ms`/`session_now`: the current `HEAD`, the
/// current time, and the promoting session (the daemon reads HEAD once).
///
/// # Errors
/// Returns the specific [`ReceiptError`] for the first failed check so the agent
/// can act on it. Fail-closed: any divergence refuses the promotion.
pub fn validate(
    r: &Receipt,
    head_now: &str,
    now_ms: i64,
    session_now: &str,
) -> Result<(), ReceiptError> {
    if !r.passed {
        return Err(ReceiptError::WitnessFailed);
    }
    if r.session_id.is_empty() || session_now.is_empty() || r.session_id != session_now {
        return Err(ReceiptError::SessionMismatch);
    }
    if r.git_head.is_empty() || r.git_head != head_now {
        return Err(ReceiptError::HeadMismatch);
    }
    // Stale (too old) OR future-dated (now < ts) — both refuse.
    let age = now_ms.saturating_sub(r.ts_ms);
    if !(0..=FRESHNESS_MS).contains(&age) {
        return Err(ReceiptError::Stale);
    }
    Ok(())
}

#[cfg(test)]
#[path = "witness_receipt_test.rs"]
mod tests;
