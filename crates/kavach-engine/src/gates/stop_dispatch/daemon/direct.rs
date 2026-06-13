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
    })
}

async fn open_state() -> Result<AppState, ()> {
    let db = kavach_surreal::open_default_resilient().await.map_err(|_| ())?;
    Ok(AppState::new(db))
}

/// Same selectors as [`super::rpc_next`], via embedded `SurrealDB`.
pub(super) fn next(method: &str, project_slug: &str) -> Result<Option<serde_json::Value>, ()> {
    let rt = tokio::runtime::Runtime::new().map_err(|_| ())?;
    rt.block_on(async {
        let state = open_state().await?;
        let params = dispatch_params(project_slug)?;
        let task = match method {
            "roadmap.next_open_task" => next_open_task(&state, params)
                .await
                .map_err(|_| ())?,
            "roadmap.next_open_hunt" => next_open_hunt(&state, params)
                .await
                .map_err(|_| ())?,
            "roadmap.promote_next_backlog" => promote_next_backlog(&state, params)
                .await
                .map_err(|_| ())?,
            _ => return Err(()),
        };
        Ok(task.as_ref().map(task_to_json))
    })
}

/// Direct census when RPC transport is down.
pub(super) fn census(project_slug: &str) -> Result<Option<(u64, u64)>, ()> {
    let rt = tokio::runtime::Runtime::new().map_err(|_| ())?;
    rt.block_on(async {
        let state = open_state().await?;
        let params = dispatch_params(project_slug)?;
        let census = open_set_census(&state, params).await.map_err(|_| ())?;
        Ok(Some((
            u64::try_from(census.runnable).map_err(|_| ())?,
            u64::try_from(census.blocked).map_err(|_| ())?,
        )))
    })
}
