// db-ops-exempt: one list query + per-row content-addressed keyed write via a
// helper (update_bandit_reward); no scan-in-loop. Mapping lives in grade.rs.
//! `db.bandit_backfill_session` RPC — the P3a JOIN that closes the RLVR loop.
//!
//! Layer-A logs each decision with `reward = None`; `r` is back-filled once the
//! 3-witness resolves. Join key = the session correlation id (on every row AND
//! known to the stop gate). When the gate learns a session's pass/fail it calls
//! this RPC, which grades that session's un-rewarded rows. Single-writer: the
//! engine never writes the reward — only the daemon, here.
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};
mod grade;
#[cfg(test)]
#[path = "bandit_backfill_test.rs"]
#[path = "bandit_backfill_test.rs"]
mod tests;
/// Back-fill request: the session to grade + whether its 3-witness verify passed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct BanditBackfillParams {
    /// The session correlation id whose un-rewarded decisions to grade (JOIN
    /// key). An opaque `sess_*` id — not a secret; logged in plaintext already.
    #[serde(rename = "session_id")]
    pub session: String,
    /// The session's 3-witness verify outcome: `true` = build+diff+tests passed.
    pub verified_clean: bool,
    /// Max rows to grade this pass (newest first). 0 ⇒ no rows.
    pub limit: u32,
}
/// The back-fill report: rows graded vs surfaced-skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct BanditBackfillResult {
    /// Whether the back-fill ran (false only on a store-load error).
    pub success: bool,
    /// Count of rows whose reward was written this pass.
    pub graded: usize,
    /// Count surfaced-skipped (malformed / no action / write failed).
    pub skipped: usize,
    /// Set on a load error; the counts are then zero.
    pub error: Option<String>,
}
/// Grade one closed session's logged decisions against its verify outcome (P3a).
///
/// # Errors
/// Returns an RPC error only on transport failure; a store-load error is reported
/// in `BanditBackfillResult.error` with `success = false`.
pub async fn bandit_backfill_session(
    ctx: &AppState,
    params: BanditBackfillParams,
) -> Result<BanditBackfillResult, ErrorObjectOwned> {
    let rows = match kavach_surreal::list_unrewarded_bandit_rows_for_session(
        &ctx.db,
        &params.session,
        params.limit,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return Ok(BanditBackfillResult {
                success: false,
                graded: 0,
                skipped: 0,
                error: Some(e.to_string()),
            });
        }
    };
    let mut graded: usize = 0;
    let mut skipped: usize = 0;
    for payload in &rows {
        if write_reward(ctx, payload, params.verified_clean).await {
            graded = graded.saturating_add(1);
        } else {
            skipped = skipped.saturating_add(1);
        }
    }
    Ok(BanditBackfillResult {
        success: true,
        graded,
        skipped,
        error: None,
    })
}
/// Label one row (via the pure `grade` map) and write its reward back; `true` on
/// success. A malformed row, unknown action, or write failure is a surfaced skip
/// (`false`) — never silently graded. The update is content-addressed by the
/// payload key, so this is a single keyed write per already-fetched row.
async fn write_reward(ctx: &AppState, payload: &str, verified_clean: bool) -> bool {
    let Some(tag) = grade::reward_tag_for_row(payload, verified_clean) else {
        return false;
    };
    kavach_surreal::update_bandit_reward(&ctx.db, payload, tag)
        .await
        .is_ok()
}
