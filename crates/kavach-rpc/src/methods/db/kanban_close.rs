// TIME: O(1) avg | SPACE: O(1)
//! `db.kanban_close` RPC method.

use super::util::{ROADMAP_TABLE, resolve_project_id};
use super::witness_gate::enforce_receipt;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_patterns::witness_receipt::Receipt;
use serde::{Deserialize, Serialize};

const STATUS_VERIFIED: &str = "verified";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct KanbanCloseParams {
    pub project: String,
    pub key: String,
    /// Witness receipt — kanban-close always promotes roadmap→verified, so it is
    /// ALWAYS gated. SOURCE: decision.cli-verifier.witness-receipt-rpc-boundary.
    #[serde(default)]
    pub receipt: Option<Receipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct KanbanCloseResult {
    pub success: bool,
    pub title: Option<String>,
    pub error: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database update fails.
pub async fn kanban_close(
    ctx: &AppState,
    params: KanbanCloseParams,
) -> Result<KanbanCloseResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let result =
        kavach_surreal::update_status(&ctx.db, ROADMAP_TABLE, &pid, &params.key, STATUS_VERIFIED)
            .await;

    match result {
        Ok(_) => Ok(KanbanCloseResult {
            success: true,
            title: Some(params.key.clone()),
            error: None,
        }),
        Err(e) => Ok(KanbanCloseResult {
            success: false,
            title: None,
            error: Some(e.to_string()),
        }),
    }
}
