//! db.delete RPC method — thin hub.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

mod types;

pub use types::{
    DeleteParams, DeleteResult, delete_confirm_phrase, delete_confirm_phrase_prefix,
};

const ERR_MULTI: &str = "'all', 'key', and 'key_prefix' are mutually exclusive";
const ERR_NEITHER: &str = "must specify 'key', 'key_prefix', or 'all: true'";

fn err(error: &str, dry_run: bool) -> DeleteResult {
    DeleteResult {
        success: false,
        deleted_count: 0,
        dry_run,
        error: Some(error.to_owned()),
    }
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when validation or database delete fails.
pub async fn delete(
    ctx: &AppState,
    params: DeleteParams,
) -> Result<DeleteResult, ErrorObjectOwned> {
    let all = params.all.unwrap_or_default();
    let dry_run = params.dry_run.unwrap_or_default();

    // Exactly one target selector: key XOR key_prefix XOR all.
    let selectors = [all, params.key.is_some(), params.key_prefix.is_some()]
        .into_iter()
        .filter(|&b| b)
        .count();
    if selectors > 1 {
        return Ok(err(ERR_MULTI, dry_run));
    }
    if selectors == 0 {
        return Ok(err(ERR_NEITHER, dry_run));
    }

    // Prefix-bound bulk purge — its own confirm/dry-run path, kept separate from
    // the single-key / whole-category branch below.
    if let Some(ref prefix) = params.key_prefix {
        return delete_prefix(ctx, &params, prefix, dry_run).await;
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

/// Prefix-bound bulk purge: delete every record in `category` whose `entry_key`
/// starts with `prefix`. Dry-run previews the count; a real run requires the
/// prefix-specific confirm phrase (so it can't be authorized by a single-key
/// confirmation of the same namespace).
async fn delete_prefix(
    ctx: &AppState,
    params: &DeleteParams,
    prefix: &str,
    dry_run: bool,
) -> Result<DeleteResult, ErrorObjectOwned> {
    if dry_run {
        let report =
            kavach_surreal::preview_delete_by_key_prefix(&ctx.db, &params.project, &params.category, prefix)
                .await
                .map_err(|e| internal(e.to_string()))?;
        return Ok(DeleteResult {
            success: true,
            deleted_count: report.count,
            dry_run: true,
            error: None,
        });
    }

    let expected = delete_confirm_phrase_prefix(&params.project, &params.category, prefix);
    if params.confirm.as_deref() != Some(expected.as_str()) {
        return Ok(DeleteResult {
            success: false,
            deleted_count: 0,
            dry_run: false,
            error: Some(types::confirmation_error_msg(&expected)),
        });
    }

    let report =
        kavach_surreal::delete_by_key_prefix(&ctx.db, &params.project, &params.category, prefix)
            .await
            .map_err(|e| internal(e.to_string()))?;
    Ok(DeleteResult {
        success: true,
        deleted_count: report.count,
        dry_run: false,
        error: None,
    })
}
