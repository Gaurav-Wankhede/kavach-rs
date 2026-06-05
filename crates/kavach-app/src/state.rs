// App-wide state via GlobalSignal.
// ALGO: GlobalSignalRegistry
// PROBLEM_CLASS: hash
// REJECTED: [{"name":"context_provider","reason":"prop drilling for app-wide state"},{"name":"thread_local","reason":"Dioxus signals manage Copy semantics; thread_local fights it"}]
// TIME: O(1) read/write per signal | SPACE: O(s) where s = signal count
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: process-global state; harder to test in isolation
// BENCHMARK: https://dioxuslabs.com/learn/0.7/essentials/basics/signals/
// SOURCE: https://docs.rs/dioxus/latest/dioxus/prelude/type.GlobalSignal.html
use dioxus::prelude::*;
use kavach_types::MemoryStatus;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq)]
pub enum Tab {
    Projects,
    Roadmap,
    Kanban,
    Decisions,
    Knowledge,
    Runs,
    Concepts,
    Mistakes,
}

impl Tab {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Projects => "Projects",
            Self::Roadmap => "Roadmap",
            Self::Kanban => "Kanban",
            Self::Decisions => "Decisions",
            Self::Knowledge => "Knowledge",
            Self::Runs => "Runs",
            Self::Concepts => "Concepts",
            Self::Mistakes => "Mistakes",
        }
    }
}

/// Lightweight summary of a single graph link, used by row badges.
#[derive(Clone, PartialEq, Eq)]
pub struct LinkSummary {
    pub rel: String,          // depends_on | blocks | supersedes | references | mentions
    pub direction: String,    // "out" or "in"
    pub target_qname: String, // <project>/<category>/<key>
}

#[derive(Clone, PartialEq, Eq)]
pub struct EntryRef {
    pub project_slug: String,
    pub category: String,
    pub key: String,
    pub title: String,
    pub content: String,
    /// Canonical status — the SAME `kavach_types::MemoryStatus` the CLI and
    /// engine write. The dashboard is a GUI wrapper over the one schema; it
    /// never invents a parallel string status domain (that drift was the
    /// root cause of the planned/category migration breakage).
    pub status: MemoryStatus,
}

/// Parse a stored `entry_status` string into the canonical `MemoryStatus`.
/// Fail-safe: an unrecognized/empty value maps to `Todo` (the schema DEFAULT)
/// with a warn — the GUI must never panic on a row the engine wrote, and the
/// canonical enum is the single source of truth for what's valid.
#[must_use]
pub fn status_from_str(s: &str) -> MemoryStatus {
    use std::str::FromStr;
    MemoryStatus::from_str(s).unwrap_or_else(|_| {
        tracing::warn!(value = s, "unknown entry_status; defaulting to todo");
        MemoryStatus::Todo
    })
}

#[derive(Clone, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Clone, PartialEq)]
pub struct RunHandle {
    pub entry_key: String,
    pub project_slug: String,
    pub branch: String,
    pub worktree_path: String,
    pub status: RunStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub events: Vec<String>,
    pub cost_usd: Option<f64>,
    /// OS pid of the spawned `claude -p` child once Running; None while
    /// Queued or after the process has exited. Read by `cancel_run`.
    pub child_pid: Option<u32>,
}

// FIX: [resource_leak + state_machine] kavach-app run-cancellation feature
// was half-built: child_pid captured but never read (no kill path) and
// RunStatus::{Queued,Cancelled} rendered but never constructed. WHY5: a
// long-running spawned child (claude -p agent loops) MUST be cancellable;
// an unkillable child is a runaway-agent + resource leak. This completes
// the feature: Queued at enqueue, child_pid retained on Running, and
// cancel_run() kills the process and transitions to Cancelled.
/// Cancel a running run: signal the OS process to terminate and mark the
/// run Cancelled. No-op if the run is not Running or has no live pid.
pub fn cancel_run(entry_key: &str) {
    let pid = {
        let runs = RUNS.read();
        runs.get(entry_key).and_then(|h| match h.status {
            RunStatus::Running => h.child_pid,
            RunStatus::Queued | RunStatus::Done | RunStatus::Failed | RunStatus::Cancelled => None,
        })
    };
    let Some(pid) = pid else { return };
    // SIGTERM via rustix (safe wrapper, no FFI/unsafe — upholds the
    // workspace forbid(unsafe_code) posture). Best-effort: a dead pid
    // just means the child already exited.
    if let Ok(p) = rustix::process::Pid::from_raw(pid.cast_signed()).ok_or(()) {
        rustix::process::kill_process(p, rustix::process::Signal::TERM).ok();
    }
    if let Some(h) = RUNS.write().get_mut(entry_key) {
        h.status = RunStatus::Cancelled;
        h.finished_at = Some(chrono::Utc::now());
        h.child_pid = None;
        h.events.push("[cancelled by user]".to_owned());
    }
}

pub static SELECTED_PROJECT: GlobalSignal<Option<String>> = Signal::global(|| None);
pub static ACTIVE_TAB: GlobalSignal<Tab> = Signal::global(|| Tab::Projects);
pub static EDITING_ENTRY: GlobalSignal<Option<EntryRef>> = Signal::global(|| None);
pub static RUN_TARGET: GlobalSignal<Option<EntryRef>> = Signal::global(|| None);
pub static RUNS: GlobalSignal<HashMap<String, RunHandle>> = Signal::global(HashMap::new);
pub static REFRESH_TICK: GlobalSignal<u64> = Signal::global(|| 0);
