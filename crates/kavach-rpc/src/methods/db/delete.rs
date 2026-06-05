// ALGO: Conditional delete + confirmation gate
//! db.delete RPC method — thin hub.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

mod types;

pub use types::{DeleteParams, DeleteResult, delete_confirm_phrase};

const ERR_BOTH: &str = "'all' and 'key' are mutually exclusive";
const ERR_NEITHER: &str = "must specify 'key' or 'all: true'";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when validation or database delete fails.
pub async fn delete(
    ctx: &AppState,
    params: DeleteParams,
) -> Result<DeleteResult, ErrorObjectOwned> {
    let all = params.all.unwrap_or_default();
    let dry_run = params.dry_run.unwrap_or_default();

    if all && params.key.is_some() {
        return Ok(DeleteResult {
            success: false,
            deleted_count: 0,
            dry_run,
            error: Some(ERR_BOTH.to_owned()),
        });
    }

    if !all && params.key.is_none() {
        return Ok(DeleteResult {
            success: false,
            deleted_count: 0,
            dry_run,
            error: Some(ERR_NEITHER.to_owned()),
        });
    }

    if dry_run {
        let report = if let Some(ref key) = params.key {
            kavach_surreal::preview_delete_by_key(&ctx.db, &params.project, &params.category, key)
                .await
                .map_err(|e| internal(e.to_string()))?
        } else {
            kavach_surreal::preview_delete_category(&ctx.db, &params.project, &params.category)
                .await
                .map_err(|e| internal(e.to_string()))?
        };
        return Ok(DeleteResult {
            success: true,
            deleted_count: report.count,
            dry_run: true,
            error: None,
        });
    }

    let expected = delete_confirm_phrase(&params.project, &params.category, params.key.as_deref());
    if params.confirm.as_deref() != Some(expected.as_str()) {
        let msg = types::confirmation_error_msg(&expected);
        return Ok(DeleteResult {
            success: false,
            deleted_count: 0,
            dry_run: false,
            error: Some(msg),
        });
    }

    let report = if let Some(ref key) = params.key {
        kavach_surreal::delete_by_key(&ctx.db, &params.project, &params.category, key)
            .await
            .map_err(|e| internal(e.to_string()))?
    } else {
        kavach_surreal::delete_category(&ctx.db, &params.project, &params.category)
            .await
            .map_err(|e| internal(e.to_string()))?
    };

    Ok(DeleteResult {
        success: true,
        deleted_count: report.count,
        dry_run: false,
        error: None,
    })
}
