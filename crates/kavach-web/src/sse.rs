//! Server-Sent Events bridge over the daemon's `change.wait` long-poll.
//!
//! The browser opens one `EventSource` to `/events`. This handler loops on
//! `change.wait(since)` — which parks server-side until the SurrealDB change
//! version advances — and emits a `refresh` event each time it does. HTMX
//! elements carrying `hx-trigger="sse:refresh from:body"` re-fetch their
//! fragment in response. This is the server-side replacement for the removed
//! Dioxus `REFRESH_TICK` signal. Errors (daemon offline) back off and retry so
//! the stream survives a daemon restart.

use std::convert::Infallible;
use std::time::Duration;

use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use futures::Stream;
use serde_json::json;

use crate::rpc::call;

/// `GET /events` — the SSE change stream.
pub async fn events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::unfold(0_u64, |since| async move {
        let (next, emit) = wait_once(since).await;
        let event = if emit {
            Event::default().event("refresh").data("1")
        } else {
            // No change (or transient error): emit a comment-only keep-alive so
            // the connection stays warm without triggering a client refetch.
            Event::default().comment("idle")
        };
        Some((Ok(event), next))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Run one `change.wait` poll. Returns `(next_since, should_emit_refresh)`.
/// On a real change the version advances and we emit; on an unchanged park or a
/// daemon-offline error we back off briefly and keep the same cursor.
async fn wait_once(since: u64) -> (u64, bool) {
    match call::<_, WaitResponse>("change.wait", json!({ "since": since })).await {
        Ok(r) if r.version > since => (r.version, true),
        Ok(r) => (r.version.max(since), false),
        Err(_) => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            (since, false)
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct WaitResponse {
    version: u64,
}
