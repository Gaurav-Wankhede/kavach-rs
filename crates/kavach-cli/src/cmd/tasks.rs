// split: intentional — single-module Tasks CLI handler, mirrors todos.rs pattern.
// ALGO: linear-scan + keyword-bag inference
// PROBLEM_CLASS: cross-project task attribution
// REJECTED: [
//   {"name":"AST parse of Claude Code TaskCreate source","reason":"Claude Code is closed; no source access"},
//   {"name":"hook into ~/.claude logs","reason":"log format unstable; not contract"},
//   {"name":"editing ~/.claude/tasks/*.json schema","reason":"Claude Code overwrites on next TaskUpdate"}
// ]
// TIME: O(t * p) where t=task count, p=project count | SPACE: O(t)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: keyword inference is heuristic — false positives possible when projects share vocabulary.
// BENCHMARK: 47 tasks * 7 projects = 329 substring checks, sub-millisecond.
// SOURCE: ~/.claude/tasks/<user>/<id>.json schema confirmed by direct inspection 2026-05.
//
//! `kavach tasks audit` — diagnose Claude Code's user-global `TaskCreate` storage.
//!
//! Background: Claude Code stores tasks at ~/.claude/tasks/<user>/<id>.json
//! with NO project field. The auto-injected "Here are the existing tasks"
//! system-reminder shows ALL tasks across every project the user works on.
//! kavach has no hook into Claude Code's task storage or reminder generation,
//! so this subcommand provides the next-best mitigation: an audit listing
//! each task's INFERRED project (via keyword match against registered project
//! slugs and paths) so operators can identify and manually clean cross-project
//! pollution.
//!
//! See roadmap.unit.task-injection-project-scope for the broader fix design.

use crate::cli::TasksAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use std::fs;
use std::path::{Path, PathBuf};

/// Minimal projection of Claude Code's ~/.claude/tasks/<user>/<id>.json schema.
/// Parsed via `serde_json::Value` to avoid adding a serde-derive dep in this thin
/// audit module — the schema is tiny and fields are read directly by key.
struct TaskFile {
    id: String,
    subject: String,
    description: String,
    status: String,
}

pub(super) fn run(action: TasksAction) -> i32 {
    match action {
        TasksAction::Audit { user } => audit(user.as_deref()),
    }
}

fn audit(user_override: Option<&str>) -> i32 {
    let dir = match resolve_tasks_dir(user_override) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let projects = load_projects();
    let tasks = match collect_tasks(&dir) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let header = format!(
        "[TASKS_AUDIT] dir={} projects={} tasks={}",
        dir.display(),
        projects.len(),
        tasks.len()
    );
    if let Err(io_err) = print_or_exit(&header) {
        return into_exit_code(io_err);
    }
    if let Err(io_err) = print_or_exit("ID    | STATUS       | INFERRED PROJECT       | SUBJECT") {
        return into_exit_code(io_err);
    }
    for task in &tasks {
        let inferred = infer_project(task, &projects);
        let line = format!(
            "{:>5} | {:<12} | {:<22} | {}",
            task.id,
            truncate(&task.status, 12),
            truncate(&inferred, 22),
            truncate(&task.subject, 60),
        );
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    0
}

fn resolve_tasks_dir(user_override: Option<&str>) -> Result<PathBuf, i32> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(e) => {
            let msg = format!("[tasks] cannot resolve HOME: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(io_err));
            }
            return Err(1);
        }
    };
    let user = match user_override {
        Some(u) => u.to_owned(),
        None => {
            if let Some(u) = infer_user_from_dir(&home) {
                u
            } else {
                if let Err(io_err) =
                    ewrite_or_exit("[tasks] cannot infer user dir; pass --user <name>")
                {
                    return Err(into_exit_code(io_err));
                }
                return Err(1);
            }
        }
    };
    Ok(PathBuf::from(home).join(".claude").join("tasks").join(user))
}

fn infer_user_from_dir(home: &str) -> Option<String> {
    let tasks_root = PathBuf::from(home).join(".claude").join("tasks");
    let entries = fs::read_dir(&tasks_root).ok()?;
    for entry in entries.flatten() {
        if entry.file_type().ok()?.is_dir() {
            return entry.file_name().to_str().map(ToOwned::to_owned);
        }
    }
    None
}

struct ProjectHint {
    slug: String,
    keywords: Vec<&'static str>,
}

fn load_projects() -> Vec<ProjectHint> {
    // We avoid a kavach-surreal dep here (would create a cycle for this thin
    // audit binary path). First cut uses a built-in slug list so the audit
    // produces actionable output today; future enhancement is to read the
    // registered project list from kavach-surreal directly.
    let known = [
        "kavach-rs",
        "nicole-carpenter",
        "ironwill",
        "astro-advisor",
        "claude-rules",
        "portfolio",
        "review-demo",
    ];
    known
        .iter()
        .map(|slug| ProjectHint {
            slug: (*slug).to_owned(),
            keywords: keywords_for_slug(slug),
        })
        .collect()
}

fn keywords_for_slug(slug: &str) -> Vec<&'static str> {
    match slug {
        "kavach-rs" => vec![
            "kavach",
            "kavach-rs",
            "kavach-cli",
            "kavach-engine",
            "kavach-rpc",
        ],
        "nicole-carpenter" => vec![
            "ironwill",
            "ironmail",
            "ironcore",
            "irongate",
            "chat-service",
            "chat-edge",
            "scylla",
            "soundbak",
            "SDUI",
            "WidgetEnvelope",
            "comms-fabric",
            "paseto",
            "introspect",
        ],
        "ironwill" => vec!["ironwill"],
        "astro-advisor" => vec!["astro-advisor", "astro_advisor"],
        "claude-rules" => vec!["claude-rules", ".claude"],
        "portfolio" => vec!["portfolio", "cloudflare-workers"],
        "review-demo" => vec!["review-demo"],
        _ => vec![],
    }
}

fn collect_tasks(dir: &Path) -> Result<Vec<TaskFile>, i32> {
    if !dir.is_dir() {
        let msg = format!("[tasks] dir not found: {}", dir.display());
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return Err(into_exit_code(io_err));
        }
        return Err(1);
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            let msg = format!("[tasks] read_dir failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(io_err));
            }
            return Err(1);
        }
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(body) = fs::read_to_string(&path) else {
            continue;
        };

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            let task = TaskFile {
                id: read_str_field(&v, "id"),
                subject: read_str_field(&v, "subject"),
                description: read_str_field(&v, "description"),
                status: read_str_field(&v, "status"),
            };
            if !task.id.is_empty() {
                out.push(task);
            }
        }
    }
    // Sort by numeric id descending so newest tasks are at top. Non-numeric
    // IDs (defensive; Claude Code IDs are always numeric strings) sort to 0,
    // grouping malformed records at the bottom — observable but not crashing.
    out.sort_by(|a, b| {
        let na: u64 = a.id.parse().unwrap_or_default();
        let nb: u64 = b.id.parse().unwrap_or_default();
        nb.cmp(&na)
    });
    Ok(out)
}

fn infer_project(task: &TaskFile, projects: &[ProjectHint]) -> String {
    let haystack = format!("{} {}", task.subject, task.description).to_lowercase();
    let mut best: Option<(&str, usize)> = None;
    for p in projects {
        let hits = p
            .keywords
            .iter()
            .filter(|kw| haystack.contains(&kw.to_lowercase()))
            .count();
        if hits > 0 && best.is_none_or(|(_, b_hits)| hits > b_hits) {
            best = Some((p.slug.as_str(), hits));
        }
    }
    match best {
        Some((slug, _)) => slug.to_owned(),
        None => "(unknown)".to_owned(),
    }
}

/// Read a string field from a `serde_json::Value`, returning the empty String
/// when the key is missing or non-string. The absence-as-empty semantic is
/// intentional for this audit: a `TaskFile` JSON missing 'subject' or 'status'
/// is malformed but we render it anyway so operators see the partial state.
fn read_str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .map_or_else(String::new, ToOwned::to_owned)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_owned();
    }
    let mut result = s.chars().take(n.saturating_sub(1)).collect::<String>();
    result.push('…');
    result
}
