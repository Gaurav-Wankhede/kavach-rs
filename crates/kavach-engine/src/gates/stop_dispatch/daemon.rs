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
    // Session identity: a card LIVE-leased by a DIFFERENT session is excluded from
    // this session's selection (multi-session task-steal fix — two terminals/tools
    // no longer grab the same card). Same env source as the lease holder
    // (`KAVACH_SESSION_ID`, set at SessionStart) so the selector's
    // `is_live_leased_by_other(me)` compares like-for-like. Empty => fail-closed
    // (any live lease is foreign), so an un-identified session never steals.
    let session_id = std::env::var("KAVACH_SESSION_ID").ok().filter(|s| !s.is_empty());
    let mut map = serde_json::Map::new();
    map.insert("project".to_owned(), serde_json::Value::String(project_slug.to_owned()));
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
            Err(_) => {}
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
    let runnable = v.get("runnable").and_then(serde_json::Value::as_u64)?;
    let blocked = v.get("blocked").and_then(serde_json::Value::as_u64)?;
    let cyclic = v.get("cyclic").and_then(serde_json::Value::as_u64).unwrap_or(0);
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
