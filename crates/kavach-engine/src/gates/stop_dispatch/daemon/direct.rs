//! Direct `SurrealDB` fallback when the RPC Unix socket is unreachable.
//!
//! Cursor hook subprocesses run sandboxed and cannot connect to
//! `~/Library/Application Support/SharedAI/kavach-rpc.sock` even while the
//! daemon is healthy. Session-start and kanban CLI already open `RocksDB`
//! directly; stop dispatch mirrors that path before emitting `SOURCE_DOWN`.
use kavach_rpc::methods::roadmap::{
    NextOpenTaskParams, NextTaskResult, next_open_hunt, next_open_task, open_set_census,
    promote_next_backlog,
};
use kavach_rpc::state::AppState;

fn dispatch_params(project_slug: &str) -> Result<NextOpenTaskParams, ()> {
    serde_json::from_value(serde_json::json!({
        "project": project_slug,
        "lane": std::env::var("KAVACH_LANE")
            .ok()
            .filter(|lane| !lane.is_empty()),
    }))
    .map_err(|_| ())
}

fn task_to_json(task: &NextTaskResult) -> serde_json::Value {
    serde_json::json!({
        "key": task.key,
        "title": task.title,
        "status": task.status,
        "content": task.content,
        "exec_prompt": task.exec_prompt,
    })
}

async fn open_state() -> Result<AppState, ()> {
    let db = kavach_surreal::open_default_resilient()
        .await
        .map_err(|_| ())?;
    Ok(AppState::new(db))
}

/// Same selectors as [`super::rpc_next`], via embedded `SurrealDB`.
pub(super) fn next(method: &str, project_slug: &str) -> Result<Option<serde_json::Value>, ()> {
    let rt = tokio::runtime::Runtime::new().map_err(|_| ())?;
    rt.block_on(async {
        let state = open_state().await?;
        let params = dispatch_params(project_slug)?;
        let task = match method {
            "roadmap.next_open_task" => next_open_task(&state, params).await.map_err(|_| ())?,
            "roadmap.next_open_hunt" => next_open_hunt(&state, params).await.map_err(|_| ())?,
            "roadmap.promote_next_backlog" => {
                promote_next_backlog(&state, params).await.map_err(|_| ())?
            }
            _ => return Err(()),
        };
        Ok(task.as_ref().map(task_to_json))
    })
}

/// E1 lease heartbeat: extend `occupied_until` for EVERY lease this session still
/// holds on an in-progress card, so a long-running tool call (a multi-minute build)
/// does NOT let the 300s lease lapse mid-work and get its card(s) reclaimed by
/// another session. Called fire-and-forget from the `PostToolUse` hook — best-effort,
/// never blocks the hook, a down DB is a silent no-op (the TTL/sweep still protects
/// correctness). Returns the count renewed (0 on any fault). Driven entirely by DB
/// state (`occupied_by` + `in_progress`), so it heartbeats the WHOLE batch a session
/// owns, not a single card.
pub(super) fn renew_my_leases() -> usize {
    let Ok(rt) = tokio::runtime::Runtime::new() else {
        return 0;
    };
    rt.block_on(async {
        let Ok(db) = kavach_surreal::open_default_resilient().await else {
            return 0;
        };
        kavach_surreal::lease::renew_active_leases(&db)
            .await
            .unwrap_or(0)
    })
}

/// Direct census when RPC transport is down. `(runnable, blocked, cyclic)`.
pub(super) fn census(project_slug: &str) -> Result<Option<(u64, u64, u64)>, ()> {
    let rt = tokio::runtime::Runtime::new().map_err(|_| ())?;
    rt.block_on(async {
        let state = open_state().await?;
        let params = dispatch_params(project_slug)?;
        let census = open_set_census(&state, params).await.map_err(|_| ())?;
        // Dispatch-reachable (roadmap-only) counts, matching `parse_census` on the
        // RPC path — never the TaskList-inflated totals, which would trap the loop.
        Ok(Some((
            u64::try_from(census.roadmap_runnable).map_err(|_| ())?,
            u64::try_from(census.roadmap_blocked).map_err(|_| ())?,
            u64::try_from(census.roadmap_cyclic).map_err(|_| ())?,
        )))
    })
}
