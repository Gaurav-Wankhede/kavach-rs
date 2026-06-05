// ALGO: Cascade delete + confirmation gate
// TIME: O(n) | SPACE: O(n)
//! `db.wipe_project` RPC method — delete all tables for project.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

mod types;

pub use types::{WipeProjectParams, WipeProjectResult, WipeReportDto, wipe_confirm_phrase};

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when validation or database wipe fails.
pub async fn wipe_project(
    ctx: &AppState,
    params: WipeProjectParams,
) -> Result<WipeProjectResult, ErrorObjectOwned> {
    let dry_run = params.dry_run.unwrap_or_default();

    if dry_run {
        let report = kavach_surreal::preview_wipe(&ctx.db, &params.project)
            .await
            .map_err(|e| internal(e.to_string()))?;
        return Ok(WipeProjectResult {
            success: true,
            report: Some(WipeReportDto {
                project_slug: report.project_slug,
                tables: report
                    .tables
                    .into_iter()
                    .map(|(t, c)| (t.to_owned(), c))
                    .collect(),
                project_deleted: report.project_deleted,
            }),
            dry_run: true,
            error: None,
        });
    }

    let expected = wipe_confirm_phrase(&params.project);
    if params.confirm.as_deref() != Some(expected.as_str()) {
        let msg = types::wipe_error_msg(&expected);
        return Ok(WipeProjectResult {
            success: false,
            report: None,
            dry_run: false,
            error: Some(msg),
        });
    }

    let report = kavach_surreal::wipe_project(&ctx.db, &params.project)
        .await
        .map_err(|e| internal(e.to_string()))?;

    Ok(WipeProjectResult {
        success: true,
        report: Some(WipeReportDto {
            project_slug: report.project_slug,
            tables: report
                .tables
                .into_iter()
                .map(|(t, c)| (t.to_owned(), c))
                .collect(),
            project_deleted: report.project_deleted,
        }),
        dry_run: false,
        error: None,
    })
}
