//! Claude Code `TaskList` store reader — the SECOND census source.
//!
//! The stop-gate census ([`super::census`]) historically counted only the
//! `SurrealDB` `roadmap` table. But the live work backlog is also kept in the
//! Claude Code `TaskList` store on disk (`~/.claude/tasks/<scope>/<id>.json`),
//! and those two stores were never reconciled: a board with 30 open `TaskList`
//! items but zero runnable `roadmap` rows reported `runnable: 0` and the gate
//! declared the queue "drained" while real work sat unseen.
//!
//! This module reads the on-disk `TaskList` store and projects it onto the same
//! runnable/blocked split the census already computes, so a single
//! [`super::census::open_set_census`] sees BOTH stores. The CC `TaskList` status
//! vocabulary (`pending | in_progress | completed`) is distinct from kavach's
//! (`todo | in_progress | done | verified`) and is mapped here at the boundary.
//!
//! SOURCE: on-disk schema verified at `~/.claude/tasks/gaurav-wankhede/*.json`
//! (fields `id, subject, description, activeForm, status, blocks, blockedBy`).

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

/// Env override for the `TaskList` store root (per workspace `§CENTRALIZED_CONFIG`).
/// Absent → derive from the home directory. Lets tests and non-default installs
/// point at a fixture/relocated store without code changes.
const TASKLIST_DIR_ENV: &str = "KAVACH_TASKLIST_DIR";

/// Public name of [`TASKLIST_DIR_ENV`] for diagnostics (e.g. the census log when
/// the store root is unresolvable). Keeps the spelling single-sourced.
pub const TASKLIST_DIR_ENV_NAME: &str = TASKLIST_DIR_ENV;

/// One on-disk Claude Code `TaskList` entry. Only the fields the census needs are
/// deserialized; unknown fields are ignored so a schema addition upstream does
/// not break the read.
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct TaskListEntry {
    /// Stable numeric id (string form), e.g. `"1286"`. Also the file stem.
    pub id: String,
    /// CC status vocabulary: `pending | in_progress | completed`.
    pub status: String,
    /// Ids this task waits on. A task is blocked while any listed id is still
    /// open (not `completed`). Mirrors roadmap `DEPENDS_ON`.
    #[serde(default)]
    pub blocked_by: Vec<String>,
}

/// Raw on-disk shape (camelCase `blockedBy`), mapped to [`TaskListEntry`].
#[derive(Debug, Deserialize)]
struct RawTaskListEntry {
    id: String,
    status: String,
    #[serde(default, rename = "blockedBy")]
    blocked_by: Vec<String>,
}

impl From<RawTaskListEntry> for TaskListEntry {
    fn from(r: RawTaskListEntry) -> Self {
        Self {
            id: r.id,
            status: r.status,
            blocked_by: r.blocked_by,
        }
    }
}

/// `true` iff a CC `TaskList` status is runnable (dispatchable work).
///
/// `pending` and `in_progress` are open work; `completed` is terminal. Any
/// non-canonical string fail-closes to non-runnable — it can never inflate the
/// runnable count from a corrupt row.
#[must_use]
pub fn is_runnable_cc_status(status: &str) -> bool {
    matches!(status, "pending" | "in_progress")
}

/// `true` iff a CC `TaskList` status is terminal/closed.
#[must_use]
fn is_closed_cc_status(status: &str) -> bool {
    status == "completed"
}

/// Resolve the `TaskList` store directory: env override, else `~/.claude/tasks`.
///
/// Returns `None` when neither the override nor a home directory is available
/// (the census then contributes zero — fail-closed, never a panic).
///
/// Reads the override from the process env at the boundary, then delegates to
/// the pure [`resolve_root`]. Kept thin so the racy global-env read stays out
/// of the testable core (Rust 2024 makes env mutation `unsafe`/unsound under
/// threads — tests drive `resolve_root` directly instead). SOURCE:
/// <https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html>
#[must_use]
pub fn tasklist_root() -> Option<PathBuf> {
    resolve_root(std::env::var_os(TASKLIST_DIR_ENV), dirs::home_dir())
}

/// Pure root resolution: explicit `override_dir` wins, else `<home>/.claude/tasks`.
///
/// `None` only when both inputs are absent. No global-state reads — fully unit
/// testable by passing the two inputs directly.
#[must_use]
pub fn resolve_root(
    override_dir: Option<std::ffi::OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(dir) = override_dir {
        return Some(PathBuf::from(dir));
    }
    home.map(|h| h.join(".claude").join("tasks"))
}

/// Count open `TaskList` entries and split them into runnable / blocked, matching
/// the roadmap census semantics.
///
/// Returns `(runnable, blocked)` where `runnable` is every open (`pending` /
/// `in_progress`) entry across all scope subdirectories, and `blocked` is the
/// subset whose `blockedBy` still references an open (non-`completed`) entry.
///
/// A missing or unreadable store contributes `(0, 0)` — the gate must never
/// crash because the `TaskList` directory is absent on a fresh machine. The
/// caller logs when the root is unresolved so a silent zero stays observable.
#[must_use]
pub fn tasklist_census(root: &std::path::Path) -> (usize, usize) {
    let entries = read_open_entries(root);
    // Index closure status across EVERY entry (open + closed) so a blockedBy
    // pointer to a completed task resolves as satisfied.
    let status_by_id: HashMap<&str, &str> = entries
        .iter()
        .map(|e| (e.id.as_str(), e.status.as_str()))
        .collect();

    let mut runnable = 0usize;
    let mut blocked = 0usize;
    for e in &entries {
        if !is_runnable_cc_status(&e.status) {
            continue;
        }
        runnable = runnable.saturating_add(1);
        if is_blocked(e, &status_by_id) {
            blocked = blocked.saturating_add(1);
        }
    }
    (runnable, blocked)
}

/// `true` iff any prerequisite of `entry` is still open.
///
/// A `blockedBy` id that is unknown to the store (already archived/deleted) is
/// treated as SATISFIED — a dangling pointer must not strand a task as
/// permanently blocked, which would forge a false `[ALL_BLOCKED]` clean-stop.
fn is_blocked(entry: &TaskListEntry, status_by_id: &HashMap<&str, &str>) -> bool {
    entry.blocked_by.iter().any(|dep| {
        status_by_id
            .get(dep.as_str())
            .is_some_and(|s| !is_closed_cc_status(s))
    })
}

/// Read every open `TaskList` entry under `root`, recursing one level into
/// per-scope subdirectories (the store is `tasks/<scope>/<id>.json`). The
/// `.archived-completed/` subdir is skipped: those are terminal and only the
/// open set drives dispatch. Unparseable files are skipped, not fatal.
fn read_open_entries(root: &std::path::Path) -> Vec<TaskListEntry> {
    let mut out = Vec::new();
    let Ok(scopes) = std::fs::read_dir(root) else {
        return out;
    };
    for scope in scopes.flatten() {
        let path = scope.path();
        if path.is_dir() {
            collect_dir(&path, &mut out);
        }
    }
    out
}

/// Collect parseable `<id>.json` files directly in `dir` (non-recursive: the
/// archive subdir is intentionally not descended into).
fn collect_dir(dir: &std::path::Path, out: &mut Vec<TaskListEntry>) {
    let Ok(files) = std::fs::read_dir(dir) else {
        return;
    };
    for file in files.flatten() {
        let path = file.path();
        if path.extension().is_some_and(|x| x == "json")
            && let Some(entry) = parse_entry(&path)
        {
            out.push(entry);
        }
    }
}

/// Parse one `TaskList` JSON file into a [`TaskListEntry`]. Returns `None` on a
/// read or parse error so a single malformed file cannot abort the census.
fn parse_entry(path: &std::path::Path) -> Option<TaskListEntry> {
    let bytes = std::fs::read(path).ok()?;
    let raw: RawTaskListEntry = serde_json::from_slice(&bytes).ok()?;
    Some(raw.into())
}

#[cfg(test)]
#[path = "tasklist_test.rs"]
mod tasklist_test;
