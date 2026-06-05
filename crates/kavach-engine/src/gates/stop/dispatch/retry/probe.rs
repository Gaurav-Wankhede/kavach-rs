//! Shared three-tier next-card probe used by every retry branch.
use crate::gates::stop_dispatch::{get_next_backlog_info, get_next_hunt_info, get_next_task_info};

/// Three-tier next-card probe: task → hunt → backlog, each tagged with its tier.
pub(super) fn next_dispatch(project: &str) -> Option<(&'static str, String, String)> {
    get_next_task_info(project)
        .map(|(k, t)| ("TASK", k, t))
        .or_else(|| get_next_hunt_info(project).map(|(k, t)| ("HUNT", k, t)))
        .or_else(|| get_next_backlog_info(project).map(|(k, t)| ("BACKLOG", k, t)))
}
