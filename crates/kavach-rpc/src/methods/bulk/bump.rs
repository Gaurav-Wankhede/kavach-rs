// bulk.sweep_apply_event — increment one of the three conformance counters.
// Wire-format `field` is a string; map to closed Rust enum at the boundary
// so the column whitelist is never user-derived.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::{ErrorCode, ErrorObjectOwned};
use kavach_surreal::bulk_manifest::{ConformanceField, bump_conformance};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BumpParams {
    pub sweep_id: String,
    /// "applied" | "refused" | "drifted" — anything else returns `InvalidParams`.
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BumpResult {
    pub ok: bool,
}

/// # Errors
///
/// Returns `InvalidParams` if the `field` is not one of "applied", "refused", or "drifted".
/// Returns an RPC error if the database operation fails.
pub async fn bump(state: &AppState, p: BumpParams) -> Result<BumpResult, ErrorObjectOwned> {
    let field = match p.field.as_str() {
        "applied" => ConformanceField::Applied,
        "refused" => ConformanceField::Refused,
        "drifted" => ConformanceField::Drifted,
        other => {
            return Err(ErrorObjectOwned::owned(
                ErrorCode::InvalidParams.code(),
                format!(
                    "bulk.sweep_apply_event: unknown field '{other}' (want applied|refused|drifted)"
                ),
                None::<()>,
            ));
        }
    };
    bump_conformance(&state.db, &p.sweep_id, field)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(BumpResult { ok: true })
}
