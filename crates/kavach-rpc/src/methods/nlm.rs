// RPC method namespace for the NanoLM live-doc corpus: store one fetched chunk,
// query the vectorless BM25 index. Mirrors methods/concept.rs (DTO-at-boundary).
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{NlmHit, nlm_query_docs, nlm_upsert_doc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct StoreParams {
    pub source_url: String,
    pub heading: String,
    pub body: String,
    pub captured_at: String,
}

/// Store one fetched doc chunk into the `nlm_doc` BM25 corpus (idempotent by
/// `source_url`+`heading`).
///
/// # Errors
/// Returns `ErrorObjectOwned` if `source_url` or `body` is empty, or the upsert fails.
pub async fn store(state: &AppState, p: StoreParams) -> Result<&'static str, ErrorObjectOwned> {
    if p.source_url.trim().is_empty() || p.body.trim().is_empty() {
        return Err(ErrorObjectOwned::owned(
            -32010,
            "nlm.store needs a non-empty source_url and body (provenance + content)",
            None::<()>,
        ));
    }
    nlm_upsert_doc(&state.db, &p.source_url, &p.heading, &p.body, &p.captured_at)
        .await
        .map_err(surreal_to_rpc)?;
    Ok("ok")
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct QueryParams {
    pub terms: String,
    pub limit: Option<usize>,
}

/// Vectorless BM25 retrieval over the `nlm_doc` corpus.
///
/// # Errors
/// Returns `ErrorObjectOwned` if the query fails.
pub async fn query(state: &AppState, p: QueryParams) -> Result<Vec<NlmHit>, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(20);
    nlm_query_docs(&state.db, &p.terms, limit)
        .await
        .map_err(surreal_to_rpc)
}
