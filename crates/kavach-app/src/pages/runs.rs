// PAGE: Runs — show currently active and recent Claude Code subprocess runs.
//
// ALGO: NewestFirstSort
// PROBLEM_CLASS: sort
// REJECTED: [{"name":"insertion_order_only","reason":"hides recent activity"},{"name":"status_grouping","reason":"adds visual clutter for small N"}]
// TIME: O(n log n) per render | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: re-sorts on every read of RUNS; n is bounded ≤ ~50
// BENCHMARK: https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_by
// SOURCE: https://dioxuslabs.com/learn/0.7
use dioxus::prelude::*;

use crate::atoms::log_view::LogView;
use crate::atoms::relative_time::RelativeTime;
use crate::state::{RUNS, RunStatus, cancel_run};

const fn status_label(s: &RunStatus) -> &'static str {
    match s {
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Done => "done",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
    }
}

const fn status_class(s: &RunStatus) -> &'static str {
    match s {
        RunStatus::Queued => "run-status run-status-queued",
        RunStatus::Running => "run-status run-status-running",
        RunStatus::Done => "run-status run-status-done",
        RunStatus::Failed => "run-status run-status-failed",
        RunStatus::Cancelled => "run-status run-status-cancelled",
    }
}

#[component]
pub(crate) fn RunsPage() -> Element {
    let runs = RUNS.read().clone();
    let mut entries: Vec<_> = runs.into_iter().collect();
    entries.sort_by_key(|x| std::cmp::Reverse(x.1.started_at));
    rsx! {
        section { class: "page page-runs",
            h1 { "Claude Code runs" }
            if entries.is_empty() {
                p { class: "hint", "No runs yet. Click ▶ on any kanban / roadmap / decision entry to start one." }
            }
            for (k, h) in entries {
                article { key: "{k}", class: "run-card",
                    header { class: "run-header",
                        span { class: status_class(&h.status), "{status_label(&h.status)}" }
                        strong { class: "run-key", "{h.entry_key}" }
                        span { class: "run-branch", "{h.branch}" }
                        RelativeTime { timestamp: h.started_at }
                        if matches!(h.status, RunStatus::Running) {
                            button {
                                class: "run-cancel",
                                "aria-label": "Cancel run",
                                onclick: {
                                    let key = k;
                                    move |_| cancel_run(&key)
                                },
                                "Cancel"
                            }
                        }
                    }
                    div { class: "run-meta",
                        code { class: "run-path", "{h.worktree_path}" }
                    }
                    LogView { lines: h.events.clone() }
                }
            }
        }
    }
}
