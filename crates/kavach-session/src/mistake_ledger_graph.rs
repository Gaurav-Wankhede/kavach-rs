// Graph-backed mistake-event path. Append-only: each block creates a
// mistake_event row, embeds it, clusters it under an anti_pattern via cosine
// kNN (threshold 0.85). hit_count is a query, not a stored counter.
//
// SINGLE-WRITER INVARIANT: this path MUST go through the kavach-rpc server,
// NOT a direct `open_default()`. The previous direct-open implementation was
// the root cause of the silently-empty mistake ledger: `record_mistake` runs
// inside an ephemeral hook child, and `open_default()` on RocksDB LOCK
// contention contends the SurrealDB server connection — from a
// short-lived child that exits before the race resolves, so nothing landed
// (proven by execution: live gate fired but the row count never moved, 2 -> 2).
// Routing the embed+append+cluster through `mistake.record` runs it inside the
// the one process that holds the SurrealDB connection.
// SOURCE: rca.mistake-ledger-dark-via-direct-open · CLAUDE.md single_writer.
use crate::Mistake;
use kavach_rpc::methods::mistake::{RecordParams, RecordResult};

const LEGACY_OPT_OUT: &str = "KAVACH_MISTAKES_LEGACY";

#[must_use]
pub fn graph_path_enabled() -> bool {
    !matches!(std::env::var(LEGACY_OPT_OUT), Ok(v) if v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Record a mistake via the kavach-rpc server (the single `RocksDB` writer).
///
/// Synchronous: `kavach_rpc::client::call` blocks on the Unix-socket round-trip,
/// so no tokio runtime is needed in the calling hook child.
///
/// # Errors
///
/// Returns an error string if the server is unreachable or the RPC itself
/// fails (embedder init, embedding, event append, or pattern clustering).
pub fn try_record_via_graph(m: &Mistake<'_>, session_id: &str) -> Result<String, String> {
    let params = RecordParams::new(
        m.gate.to_owned(),
        m.banned_sample.to_owned(),
        m.correct_action.to_owned(),
        session_id.to_owned(),
        Some(m.project.to_owned()),
    );
    kavach_rpc::client::call::<_, RecordResult>("mistake.record", Some(params))
        .map(|r| r.ids)
        .map_err(|e| format!("{e:?}"))
}
