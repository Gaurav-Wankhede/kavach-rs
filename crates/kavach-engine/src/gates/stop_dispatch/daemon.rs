//! RPC daemon self-heal: launchd respawn (`spawn`) + bounded poll-retry probe.
mod spawn;

use spawn::try_spawn_rpc_daemon;

/// RPC ok+task -> `Ok(Some(json))` · ok+empty -> `Ok(None)` ·
/// transport error -> `Err(())` (caller fails closed via sentinel).
///
/// SELF-HEAL: on the first transport error, attempt a one-shot daemon respawn
/// and poll-retry (bounded 20×500ms) while `RocksDB` cold-opens, then honest
/// `Err(())` — the bounded escape valve for the fail-closed sentinel.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(super) fn rpc_next(method: &str, project_slug: &str) -> Result<Option<serde_json::Value>, ()> {
    let dbg = std::env::var("KAVACH_RPC_CLIENT_DEBUG").is_ok();
    let params = serde_json::json!({ "project": project_slug });
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
        return Err(());
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
    Err(())
}
