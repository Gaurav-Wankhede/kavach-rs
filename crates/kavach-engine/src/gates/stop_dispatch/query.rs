//! Next-dispatchable selectors: task / hunt / backlog, with fail-closed sentinel.
use super::card::SOURCE_DOWN_KEY;
use super::daemon::rpc_next;

fn source_down_sentinel() -> (String, String) {
    (
        SOURCE_DOWN_KEY.to_owned(),
        "kanban source UNREACHABLE (RPC + direct DB both failed) — cannot \
         verify empty; assume work pending, do NOT stop"
            .to_owned(),
    )
}

fn label_from(val: &serde_json::Value) -> Option<(String, String)> {
    let key = val.get("key").and_then(|s| s.as_str())?.to_owned();
    let title = val.get("title").and_then(|s| s.as_str())?.to_owned();
    let status = val
        .get("status")
        .and_then(|s| s.as_str())
        .map_or_else(String::new, str::to_owned);
    Some((key, format!("{title} [{status}]")))
}

/// Next dispatchable roadmap task (key, `title [status]`), or None when empty.
/// `SOURCE_DOWN` sentinel on RPC outage (caller fails closed).
pub(crate) fn get_next_task_info(project_slug: &str) -> Option<(String, String)> {
    select(project_slug, "roadmap.next_open_task")
}

/// Next open bug-hunt card (roadmap row, key prefix 'hunt.', open status).
/// The harness cannot stop while a proven, unfixed defect remains.
pub(crate) fn get_next_hunt_info(project_slug: &str) -> Option<(String, String)> {
    select(project_slug, "roadmap.next_open_hunt")
}

/// Continuous-pipeline refill: priority-ordered runnable backlog head, so the
/// loop keeps draining the roadmap rather than halting on an empty open-set.
pub(crate) fn get_next_backlog_info(project_slug: &str) -> Option<(String, String)> {
    select(project_slug, "roadmap.promote_next_backlog")
}

/// Shared selector body: empty slug → None; RPC down → fail-closed sentinel.
fn select(project_slug: &str, method: &str) -> Option<(String, String)> {
    if project_slug.is_empty() {
        return None;
    }
    match rpc_next(method, project_slug) {
        Ok(Some(v)) => label_from(&v),
        Ok(None) => None,
        Err(()) => Some(source_down_sentinel()), // RPC down -> fail closed
    }
}
