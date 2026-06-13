//! RPC daemon self-heal: launchd respawn (`spawn`) + bounded poll-retry probe.
//! When RPC stays unreachable (e.g. Cursor hook sandbox blocks the Unix socket),
//! fall back to direct `SurrealDB` — same resilient open as session-start.
mod direct;
mod spawn;

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
    let mut map = serde_json::Map::new();
    map.insert("project".to_owned(), serde_json::Value::String(project_slug.to_owned()));
    if let Some(l) = lane {
        map.insert("lane".to_owned(), serde_json::Value::String(l));
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
        .map_or_else(
            |_| direct::census(project_slug),
            |v| {
                let runnable = v.get("runnable").and_then(serde_json::Value::as_u64);
                let blocked = v.get("blocked").and_then(serde_json::Value::as_u64);
                // Absent `cyclic` (older daemon) defaults to 0 — backward-compatible.
                let cyclic = v.get("cyclic").and_then(serde_json::Value::as_u64).unwrap_or(0);
                Ok(runnable.zip(blocked).map(|(r, b)| (r, b, cyclic)))
            },
        )
}
