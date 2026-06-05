//! Edge projection for `db.write` — the daemon-side half of the single-writer
//! relationship split.
//!
//! Extraction is the CLI's job (it depends on `kavach-engine`; the daemon cannot
//! — that would cycle); projection is the daemon's (`kavach-surreal`), so the two
//! never both hold a `RocksDB` handle. SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use super::WriteParams;
use crate::state::AppState;

/// Project the CLI-extracted, already-normalised edges as graph relationships
/// inside the daemon (the single writer). Best-effort: a projection failure is
/// logged, never fails the write — the row already committed.
pub(super) async fn project_relationships(ctx: &AppState, params: &WriteParams, qname: &str) {
    if params.relationships.is_empty() {
        return;
    }
    if let Err(e) =
        kavach_surreal::upsert_relationships(&ctx.db, qname, &params.relationships).await
    {
        tracing::warn!("db.write: relationship projection failed for {qname}: {e}");
    }
}
