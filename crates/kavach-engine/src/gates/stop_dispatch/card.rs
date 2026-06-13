//! Card-state predicates: open-check, atomic claim, saturation breaker.
//!
//! Fail-CLOSED on RPC transport error — an unobservable status is treated as
//! still-open so enforcement keeps running rather than silently abandoning
//! in-progress work on a daemon outage (CWE-392 loop-disabling variant).

/// Sentinel key returned when the kanban source-of-truth is unreachable. The
/// caller BLOCKS the stop (fail-closed) rather than reading a phantom "empty".
/// Fail-closed only when RPC **and** direct `SurrealDB` both miss (see
/// `daemon::direct`). A wrong "no work" answer is worse than an error.
pub(crate) const SOURCE_DOWN_KEY: &str = "__kanban_source_unreachable__";

/// Query the current `entry_status` for a card. `None` on RPC miss or absent key.
pub(crate) fn card_entry_status(project_slug: &str, key: &str) -> Option<String> {
    if project_slug.is_empty() || key.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug, "key": key});
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.entry_status", Some(params))
        .ok()
        .and_then(|v| {
            v.get("status")
                .and_then(|s| s.as_str())
                .map(str::to_owned)
        })
}

/// Query kavach DB for whether a card is still open (dispatch-runnable).
/// SOURCE: `schema_v10` — strict `entry_status` enum.
pub(crate) fn card_is_still_open(project_slug: &str, key: &str) -> bool {
    if project_slug.is_empty() || key.is_empty() {
        return false;
    }
    let params = serde_json::json!({"project": project_slug, "key": key});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("roadmap.entry_status", Some(params));
    let status = match result {
        Ok(v) => v
            .get("status")
            .and_then(|s| s.as_str())
            .map(ToOwned::to_owned),
        // FIX [silent_failure / CWE-392]: RPC transport error is NOT "card
        // closed". Conflating an unreachable source-of-truth with a definitive
        // negative makes callers clear the live card and skip their block,
        // silently abandoning in-progress work. Fail CLOSED — still-open.
        Err(_) => return true,
    };
    // Only `todo`/`in_progress` are dispatch-runnable; `done` awaits manual
    // `verified` promotion and `verified`/`deferred` are terminal — treat both
    // as clearable so the stale-pointer check stops looping.
    matches!(status.as_deref(), Some("todo" | "in_progress"))
}

/// Atomically CLAIM the dispatched card (`todo -> in_progress`). Best-effort: a
/// transport miss degrades to prior behavior (card stays `todo`, re-dispatched
/// next turn), never a stalled loop. Idempotent. True iff the row flipped now.
pub(crate) fn claim_card(project_slug: &str, key: &str) -> bool {
    if project_slug.is_empty() || key.is_empty() || key == SOURCE_DOWN_KEY {
        return false;
    }
    let params = serde_json::json!({ "project": project_slug, "key": key });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.claim_card", Some(params))
        .ok()
        .and_then(|v| v.get("claimed").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

/// True iff a single in-flight card is SATURATED — the re-block breaker is past
/// its spin ceiling AND no progress (file/DB write) happened since the last
/// Stop. Only then may the terminal proceed despite a non-empty queue, so one
/// stuck card cannot wedge the session forever. While progress is being made
/// this is FALSE no matter how high the breaker climbed.
pub(crate) const fn is_backlog_saturated(
    stop_reblock_count: i32,
    has_progress_since_last_stop: bool,
) -> bool {
    stop_reblock_count > kavach_session::SessionState::max_stop_reblocks()
        && !has_progress_since_last_stop
}
