// bulk.sweep_close — terminal status for a manifest. `reason` chooses between
// `closed` (agent finished) and `expired` (TTL fired). Audit trail preserves
// the distinction; stop-gate refuses clean stop while any active manifest remains.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::{ErrorCode, ErrorObjectOwned};
use kavach_surreal::bulk_manifest::{close as bm_close, mark_expired};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CloseParams {
    pub sweep_id: String,
    /// "closed" | "expired" — anything else returns `InvalidParams`.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CloseResult {
    pub ok: bool,
    pub final_status: String,
}

/// # Errors
/// Returns `InvalidParams` if `reason` is not "closed" or "expired".
pub async fn close(state: &AppState, p: CloseParams) -> Result<CloseResult, ErrorObjectOwned> {
    let final_status = match p.reason.as_str() {
        "closed" => {
            bm_close(&state.db, &p.sweep_id)
                .await
                .map_err(surreal_to_rpc)?;
            "closed".to_owned()
        }
        "expired" => {
            mark_expired(&state.db, &p.sweep_id)
                .await
                .map_err(surreal_to_rpc)?;
            "expired".to_owned()
        }
        other => {
            return Err(ErrorObjectOwned::owned(
                ErrorCode::InvalidParams.code(),
                format!("bulk.sweep_close: unknown reason '{other}' (want closed|expired)"),
                None::<()>,
            ));
        }
    };
    Ok(CloseResult {
        ok: true,
        final_status,
    })
}
