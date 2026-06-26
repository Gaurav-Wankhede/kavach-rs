//! RPC daemon self-heal: launchd respawn (`spawn`) + bounded poll-retry probe.
//! When RPC stays unreachable (e.g. Cursor hook sandbox blocks the Unix socket),
//! fall back to direct `SurrealDB` — same resilient open as session-start.
mod direct;
mod directive;
mod spawn;

pub(super) use directive::rpc_get_directive;
use spawn::try_spawn_rpc_daemon;

/// RPC ok+task -> `Ok(Some(json))` · ok+empty -> `Ok(None)` ·
/// transport error -> `Err(())` (caller fails closed via sentinel).
///
/// SELF-HEAL: on the first transport error, attempt a one-shot daemon respawn
/// and poll-retry (bounded 20×500ms) while `RocksDB` cold-opens. When spawn is
/// blocked (hook sandbox) or retries exhaust, fall back to direct `SurrealDB`.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(super) fn rpc_next(method: &str, project_slug: &str) -> Result<Option<serde_json::Value>, ()> {
    let dbg = std::env::var("KAVACH_RPC_CLIENT_DEBUG").is_ok();
    // Lane-affinity sharding: a session running `KAVACH_LANE=<name>` dispatches
    // its own lane first, then the unlaned backlog, never a foreign lane (the
    // two-pass logic lives in roadmap::next_open_task). Unset/empty lane => the
    // field is absent and dispatch sees the whole project backlog as before.
    let lane = std::env::var("KAVACH_LANE").ok().filter(|l| !l.is_empty());
    // Session-lease isolation: prevent multi-session task steal via KAVACH_SESSION_ID.
    // See decision.engine.session-lease-isolation.
    let session_id = Some(kavach_session::resolved_session_id()).filter(|s| !s.is_empty());
    let mut map = serde_json::Map::new();
    map.insert(
        "project".to_owned(),
        serde_json::Value::String(project_slug.to_owned()),
    );
    if let Some(l) = lane {
        map.insert("lane".to_owned(), serde_json::Value::String(l));
    }
    if let Some(s) = session_id {
        map.insert("session_id".to_owned(), serde_json::Value::String(s));
    }
    let params = serde_json::Value::Object(map);
    let classify = |v: serde_json::Value| {
        if v.is_object() && v.get("key").is_some() {
            Ok(Some(v))
        } else {
            Ok(None)
        }
    };
    let first = kavach_rpc::client::call::<_, serde_json::Value>(method, Some(params.clone()));
    if dbg {
        match &first {
            Ok(v) => eprintln!("[rpc_next] {method}: first OK = {v}"),
            Err(e) => eprintln!("[rpc_next] {method}: first ERR = {e:?}"),
        }
    }
    if let Ok(v) = first {
        return classify(v);
    }
    // Daemon unreachable — self-heal: spawn it, then poll-retry while RocksDB
    // cold-opens (~2-4s). Bounded: 20 attempts × 500ms = 10s.
    let spawned = try_spawn_rpc_daemon();
    if dbg {
        eprintln!("[rpc_next] try_spawn_rpc_daemon -> {spawned}");
    }
    if !spawned {
        if dbg {
            eprintln!("[rpc_next] spawn unavailable — trying direct `SurrealDB`");
        }
        return direct::next(method, project_slug);
    }
    for attempt in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        match kavach_rpc::client::call::<_, serde_json::Value>(method, Some(params.clone())) {
            Ok(v) => {
                if dbg {
                    eprintln!("[rpc_next] retry {attempt} OK after spawn");
                }
                return classify(v);
            }
            Err(e) if dbg => eprintln!("[rpc_next] retry {attempt} ERR = {e:?}"),
            Err(_) => {} // doctor:ok retry fallthrough — logged when dbg; falls to direct-DB below
        }
    }
    if dbg {
        eprintln!("[rpc_next] {method}: RPC exhausted — trying direct SurrealDB");
    }
    direct::next(method, project_slug)
}

/// RPC-ONLY census: a single bounded `kavach_rpc::client::call` (`UnixStream`
/// connect + 2s socket timeout) with NO daemon self-heal and NO direct-DB
/// fallback. Returns `None` instantly on ANY transport error.
///
/// WHY a separate path: [`rpc_open_census`] falls back to `direct::census`,
/// which cold-opens the embedded `RocksDB` (~seconds, and contends the live app's
/// DB lock). That is safe ONLY on the Stop gate's already-drained branch, where
/// a daemon spawned by the prior `rpc_next` self-heal is warm. Hot entry hooks
/// (`SessionStart` / `UserPromptSubmit`) run every turn with NO warm-daemon
/// guarantee — routing them through the heavy fallback blocks the hook (observed
/// as a nextest SIGTERM). These hooks must fail SOFT and FAST to the legacy nag,
/// never block the session on a DB cold-open. See the parsing in [`parse_census`].
pub(super) fn rpc_census_only(project_slug: &str) -> Option<(u64, u64, u64)> {
    let params = serde_json::json!({ "project": project_slug });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.open_set_census", Some(params))
        .ok()
        .and_then(|v| parse_census(&v))
}

/// E1 lease heartbeat (crate-visible): extend `occupied_until` for every lease this
/// session still holds on an in-progress card. Best-effort fire-and-forget from the
/// `PostToolUse` hook — direct DB (the hook subprocess may be sandboxed off the RPC
/// socket, like the other direct paths). Returns the count renewed (0 on any fault).
pub(crate) fn renew_my_leases() -> usize {
    direct::renew_my_leases()
}

/// RPC-ONLY next-task name: bounded single call, no self-heal, no direct DB.
/// `None` on outage so the caller omits the "next card" line rather than block.
pub(super) fn rpc_next_only(project_slug: &str) -> Option<serde_json::Value> {
    let params = serde_json::json!({ "project": project_slug });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.next_open_task", Some(params))
        .ok()
        .filter(|v| v.is_object() && v.get("key").is_some())
}

/// Extract `(runnable, blocked, cyclic)` from a census RPC value. Absent `cyclic`
/// (older daemon) defaults to 0; a missing `runnable`/`blocked` yields `None`.
fn parse_census(v: &serde_json::Value) -> Option<(u64, u64, u64)> {
    // Prefer dispatch-reachable `roadmap_*` counts over the TaskList-inflated
    // totals (else a global open task traps any project session). Falls back to
    // totals for an old daemon. See decision.harness.census-split-roadmap-vs-tasklist.
    let runnable = v
        .get("roadmap_runnable")
        .or_else(|| v.get("runnable"))
        .and_then(serde_json::Value::as_u64)?;
    let blocked = v
        .get("roadmap_blocked")
        .or_else(|| v.get("blocked"))
        .and_then(serde_json::Value::as_u64)?;
    let cyclic = v
        .get("roadmap_cyclic")
        .or_else(|| v.get("cyclic"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    Some((runnable, blocked, cyclic))
}

/// `roadmap.open_set_census` → `(runnable, blocked, cyclic)` counts, or `Err(())`
/// on a transport error (caller fails closed). `cyclic` = runnable cards whose
/// declared deps form a cycle — they can never satisfy deps, so a non-zero count
/// must REFUSE a clean-stop (else a deadlock forges a false `[ALL_BLOCKED]`).
/// Single-shot: the board census is only consulted on the already-drained branch,
/// where a daemon spawned by the prior `rpc_next` self-heal is already warm; an
/// outage here degrades to "do not clean-stop", never a wrong "board empty".
pub(super) fn rpc_open_census(project_slug: &str) -> Result<Option<(u64, u64, u64)>, ()> {
    let params = serde_json::json!({ "project": project_slug });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.open_set_census", Some(params))
        .map_or_else(|_| direct::census(project_slug), |v| Ok(parse_census(&v)))
}

#[cfg(test)]
mod tests {
    use super::parse_census;

    #[test]
    fn prefers_roadmap_only_counts_over_tasklist_inflated_totals() {
        // The real-world bug: top-level totals fold the GLOBAL TaskList (21,1),
        // but only the roadmap subset (0,0) is dispatch-reachable in this lane.
        // The gate MUST see the roadmap-only counts or it traps the loop forever.
        let v = serde_json::json!({
            "runnable": 21, "blocked": 1, "cyclic": 0,
            "roadmap_runnable": 0, "roadmap_blocked": 0, "roadmap_cyclic": 0,
        });
        assert_eq!(parse_census(&v), Some((0, 0, 0)));
    }

    #[test]
    fn falls_back_to_totals_for_old_daemon_without_roadmap_fields() {
        // An older daemon payload lacks `roadmap_*`; degrade to the totals so the
        // gate still observes a board rather than panicking or zeroing.
        let v = serde_json::json!({ "runnable": 3, "blocked": 1, "cyclic": 0 });
        assert_eq!(parse_census(&v), Some((3, 1, 0)));
    }

    #[test]
    fn roadmap_remainder_survives_when_real_local_work_exists() {
        // A genuine kavach-rs roadmap todo (2 runnable, 0 blocked) must still be
        // seen as a dispatchable remainder — the original clean-stop fix intact.
        let v = serde_json::json!({
            "runnable": 23, "blocked": 1, "cyclic": 0,
            "roadmap_runnable": 2, "roadmap_blocked": 0, "roadmap_cyclic": 0,
        });
        assert_eq!(parse_census(&v), Some((2, 0, 0)));
    }
}
