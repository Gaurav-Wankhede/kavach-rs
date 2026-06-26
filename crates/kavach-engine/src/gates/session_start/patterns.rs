//! Self-evolve context injection: hot gate-patterns + the mistake ledger +
//! the learned advisory policy. The K-PRI-ranked mistake ledger lives in the
//! `mistakes` submodule; the RLVR-learned policy in `policy`.
mod mistakes;
mod policy;

use std::fmt::Write as _;

pub(super) use mistakes::mistake_ledger_context;
pub(super) use policy::learned_policy_context;

/// Candidate window pulled from the daemon before relevance-gating; `k_pri`
/// re-ranks this set and only the top-K survive into the prompt.
const HOT_PATTERN_WINDOW: i64 = 30;
/// Token-discipline cap (global directive §7): inject at most this many ranked
/// patterns so the frame stays within the per-turn injection budget.
const HOT_PATTERN_TOP_K: usize = 3;

/// Load autonomous gate patterns and inject the top-K ranked by `k_pri`
/// (recency × recurrence) — NOT raw frequency or arbitrary DB order. This is the
/// `ExpeL`→`ERL` fix: a frequency/unordered dump becomes selective top-k
/// retrieval at session start so Claude sees the most relevant cached fixes
/// without waiting for a tool failure. Reuses `kavach_patterns::k_pri` (see
/// `decision.loop-eng.reuse-k-pri-not-new-scorer`). Returns `None` if the DB is
/// unavailable or no autonomous patterns exist.
pub(super) fn hot_pattern_context(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug, "limit": HOT_PATTERN_WINDOW});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("gate_pattern.list_hot", Some(params));
    let Ok(serde_json::Value::Array(patterns)) = result else {
        return None;
    };
    if patterns.is_empty() {
        return None;
    }
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // Build (display-line, K-PRI signal) per candidate. `occurrence_count` is
    // the recurrence (LFU) axis; age since `updated_unix` is the recency-decay
    // axis. A missing `updated_unix` (legacy row) is treated as fresh (age 0) so
    // it is ranked on recurrence alone, never unfairly buried.
    let rows: Vec<(String, kavach_patterns::k_pri::Signals)> = patterns
        .iter()
        .map(|p| {
            let tokens = p.get("error_tokens").and_then(|v| v.as_str()).unwrap_or("");
            let fix = p.get("fix_strategy").and_then(|v| v.as_str()).unwrap_or("");
            let n = p
                .get("occurrence_count")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let updated = p
                .get("updated_unix")
                .and_then(serde_json::Value::as_i64)
                .and_then(|u| u64::try_from(u).ok())
                .unwrap_or(now_unix);
            let sig = kavach_patterns::k_pri::Signals {
                hit_count: u32::try_from(n).unwrap_or(u32::MAX),
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "age in whole days; f64 represents every u64/86_400 result exactly"
                )]
                age_days: now_unix.saturating_sub(updated).saturating_div(86_400) as f64,
                ..Default::default()
            };
            (
                format!("pattern: {tokens} | fix: {fix} | occurrences: {n}"),
                sig,
            )
        })
        .collect();
    let scored = kavach_patterns::k_pri::rank(
        rows,
        kavach_patterns::k_pri::W_MISTAKE_LEDGER,
        |(_, sig)| *sig,
    );
    let mut ctx = String::from(
        "\n[SELF_EVOLVE_PATTERNS]\nstatus: autonomous (K-PRI ranked: recency × recurrence)\n",
    );
    for ((line, _sig), s) in scored.iter().take(HOT_PATTERN_TOP_K) {
        writeln!(ctx, "[pri={s:.2}] {line}").ok();
    }
    Some(ctx)
}
