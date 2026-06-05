// LIVE SELECT watcher — the event source behind the GUI's live updates.
//
// Spawns one background task that runs a SurrealDB LIVE SELECT on each table
// the GUI renders. Every committed Create/Update/Delete yields a notification;
// the task bumps the shared `ChangeFeed`, which wakes any `change.wait` RPC
// caller (the GUI long-poll). No polling — the DB pushes.
//
// Engine note: LIVE SELECT works on the embedded RocksDB engine (our default),
// is single-node (we are one daemon), and fires only on committed transactions.
// SOURCE: surrealdb.com/docs/sdk/rust/concepts/live · research.poll-vs-event-gui.
use crate::state::ChangeFeed;
use futures::StreamExt;
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::method::Stream;
use surrealdb_types::Value;

/// Tables whose changes should push a refresh. The first five are the rows the
/// GUI renders. `depends_on`/`blocks` are the DAG dependency-edge tables: a new
/// edge (e.g. a `kavach db write --depends-on …`) changes the scheduler's
/// ready-set and critical-path ordering, so the scheduler must re-tick the
/// instant an edge lands — not on the next poll. Row status flips already
/// arrive via `roadmap`; this closes the reactive loop for edge mutations too.
const WATCHED_TABLES: &[&str] = &[
    "decision",
    "roadmap",
    "research",
    "pattern",
    "event",
    "depends_on",
    "blocks",
];

/// Spawn the LIVE SELECT watcher. Returns immediately; the task runs for the
/// daemon's lifetime.
///
/// If a live query cannot be established it logs and skips that table — a
/// watcher failure must never take the daemon down (fail-open for liveness:
/// the GUI falls back to its manual refresh button).
pub fn spawn(db: Arc<Surreal<Db>>, changes: Arc<ChangeFeed>) {
    tokio::spawn(async move {
        let mut streams: Vec<Stream<Vec<Value>>> = Vec::new();
        for table in WATCHED_TABLES {
            match db.select(*table).live().await {
                Ok(stream) => streams.push(stream),
                Err(e) => {
                    tracing::warn!("live_watch: LIVE SELECT on `{table}` failed: {e}");
                }
            }
        }

        if streams.is_empty() {
            tracing::warn!(
                "live_watch: no live streams established; GUI will rely on manual refresh"
            );
            return;
        }

        // Merge every table's notification stream into one. Each item is a
        // committed change on some watched table — we only need the signal,
        // not the payload, so a bump per notification is sufficient.
        let mut merged = futures::stream::select_all(streams);
        tracing::info!(
            "live_watch: watching {} tables for changes",
            WATCHED_TABLES.len()
        );
        while let Some(notification) = merged.next().await {
            match notification {
                Ok(_) => changes.bump(),
                Err(e) => tracing::warn!("live_watch: notification error: {e}"),
            }
        }
        tracing::warn!("live_watch: all live streams closed; live updates stopped");
    });
}
