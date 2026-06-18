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

/// The session id holding a LIVE lease on `roadmap:key`, or `None` if the lease
/// is unheld/expired (the `lease.status` RPC already filters `occupied_until >
/// now`, so any `Some` is a live holder). On RPC miss returns `None` — fail
/// OPEN here is correct: a lease we cannot observe must NOT block a resume, else
/// a transient daemon blip would strand the owner's own card. The owner-vs-foreign
/// decision is made by the caller comparing this against its own session id.
pub(crate) fn live_lease_holder(key: &str) -> Option<String> {
    if key.is_empty() || key == SOURCE_DOWN_KEY {
        return None;
    }
    let params = serde_json::json!({ "table": "roadmap", "key": key });
    // `lease.status` => the live Lease object, or JSON null when unheld/expired.
    // A null/absent `session_id` is "no live holder" — exactly the fall-through-OK
    // case. RPC miss (`.ok()` => None) is also treated as no holder (fail OPEN).
    kavach_rpc::client::call::<_, serde_json::Value>("lease.status", Some(params))
        .ok()
        .and_then(|v| {
            v.get("session_id")
                .and_then(serde_json::Value::as_str)
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
    status
        .as_deref()
        .and_then(|s| s.parse::<kavach_types::MemoryStatus>().ok())
        .is_some_and(kavach_types::MemoryStatus::is_runnable)
}

// PARKING ABOLISHED (owner directive 2026-06-16, reaffirmed 2026-06-17): there is
// no `card_is_honestly_parked` stop-gate escape. The former `AGENT_BLOCKED:`/
// `OWNER-GATED:` content markers no longer escape the non-surrenderable
// close-before-advance block. A card is either CLOSED (done/verified, 3-witness)
// or DELETED (`kavach db delete --category roadmap --key ...`) — never marker-parked. The only honest
// exits from the close block are now a real status-update or deletion of the card,
// per global CLAUDE.md `§delete_not_park`.

/// Atomically CLAIM the dispatched card (`todo -> in_progress`). Best-effort: a
/// transport miss degrades to prior behavior (card stays `todo`, re-dispatched
/// next turn), never a stalled loop. Idempotent. True iff the row flipped now.
pub(crate) fn claim_card(project_slug: &str, key: &str) -> bool {
    if project_slug.is_empty() || key.is_empty() || key == SOURCE_DOWN_KEY {
        return false;
    }
    // Carry this session's id so the RPC fuses an occupancy lease (owner+TTL+
    // fence) onto the won claim — without an owner a hung session's card cannot
    // be told from a crashed one's, and a 2nd live session would resume it. The
    // id source matches `mistake_ledger.rs`/`env_session_id`: the env var set by
    // the Claude Code / Cursor hook edge. Absent (legacy) => status-only claim.
    let session_id = kavach_session::resolved_session_id();
    let params = serde_json::json!({
        "project": project_slug,
        "key": key,
        "session_id": session_id,
    });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.claim_card", Some(params))
        .ok()
        .and_then(|v| v.get("claimed").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

