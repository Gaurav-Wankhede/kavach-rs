// split: intentional — single command handler module, not handlers
// TIME: O(n) bytes scanned + O(m) keys diffed | SPACE: O(m) HashMap entries
// YEAR: 2026 | SEARCHED: 2026-05
//! `kavach todos sync` — extract `kavach_todo`!() macros from source files
//! and synchronize them with kanban roadmap entries.

use crate::cli::TodosAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const SKIP_DIRS: [&str; 4] = ["target", "node_modules", "dist", ".git"];

fn todo_regex() -> Result<Regex, regex::Error> {
    Regex::new(r#"(?s)kavach_todo!\s*\(\s*"((?:\\.|[^"\\])+)""#)
}

pub(super) fn run(action: TodosAction) -> i32 {
    match action {
        TodosAction::Sync {
            project,
            path,
            dry_run,
        } => sync(&project, &path, dry_run),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodoHit {
    file: String,
    line: usize,
    description: String,
}

/// Collision-safe key: BLAKE3-hashed file path + line number.
/// `replace()` was insufficient — `src/foo.rs` and `src_foo/rs.rs` both became `src_foo_rs`.
fn key_for(hit: &TodoHit) -> String {
    let hash = blake3::hash(hit.file.as_bytes()).to_hex();
    let short: String = hash.chars().take(12).collect();
    format!("todo.{}.{}", short, hit.line)
}

fn sync(project: &str, root_path: &str, dry_run: bool) -> i32 {
    let root = Path::new(root_path);
    let mut hits: HashMap<String, TodoHit> = HashMap::new();
    let scanned = walk_and_count(root, root, &mut hits);

    let header = format!(
        "kavach todos: scanned {scanned} files, found {} kavach_todo!() invocations",
        hits.len()
    );
    if let Err(io_err) = print_or_exit(&header) {
        return into_exit_code(io_err);
    }

    if dry_run {
        for (k, h) in &hits {
            let line = format!("  [{k}] {}:{} {}", h.file, h.line, h.description);
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
        return 0;
    }

    // Orphan cleanup: list existing todo.* roadmap rows, delete those not in current scan.
    let existing_keys = list_existing_todo_keys(project);
    let mut removed = 0usize;
    for key in &existing_keys {
        if !hits.contains_key(key)
            && let Ok(s) = std::process::Command::new("kavach")
                .args([
                    "db",
                    "delete",
                    "--project",
                    project,
                    "--category",
                    "roadmap",
                    "--key",
                    key,
                    "--confirm",
                ])
                .status()
            && s.success()
        {
            removed = removed.saturating_add(1);
            let line = format!("  - {key} (orphaned)");
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
    }

    let mut written = 0usize;
    for (key, hit) in &hits {
        let title: String = hit.description.chars().take(60).collect();
        let content = format!(
            "File: {}:{}\nDescription: {}\nType: kavach_todo",
            hit.file, hit.line, hit.description
        );
        let status = std::process::Command::new("kavach")
            .args([
                "db",
                "write",
                "--project",
                project,
                "--category",
                "roadmap",
                "--key",
                key,
                "--title",
                &title,
                "--content",
                &content,
            ])
            .status();
        match status {
            Ok(s) if s.success() => {
                written = written.saturating_add(1);
                let line = format!("  + {key}");
                if let Err(io_err) = print_or_exit(&line) {
                    return into_exit_code(io_err);
                }
            }
            _ => {
                let msg = format!("  ! failed to write {key}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
            }
        }
    }

    let summary =
        format!("kavach todos: sync complete ({written} written, {removed} orphans removed)");
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    0
}

/// List existing todo.* roadmap keys for the project via subprocess.
/// Returns empty vec on any error (best-effort orphan cleanup).
fn list_existing_todo_keys(project: &str) -> Vec<String> {
    let output = std::process::Command::new("kavach")
        .args(["db", "query", "--project", project, "--category", "roadmap"])
        .output();
    let Ok(out) = output else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    let stdout_str = String::from_utf8_lossy(&out.stdout);
    let mut keys = Vec::new();
    for line in stdout_str.lines() {
        // Format: "[roadmap] todo.<hash>.<line> — <title>"
        let Some(rest) = line.strip_prefix("[roadmap] ") else {
            continue;
        };
        let Some(key) = rest.split_whitespace().next() else {
            continue;
        };
        if key.starts_with("todo.") {
            keys.push(key.to_owned());
        }
    }
    keys
}

fn walk_and_count(root: &Path, dir: &Path, hits: &mut HashMap<String, TodoHit>) -> usize {
    let mut count = 0usize;
    let Ok(entries) = fs::read_dir(dir) else {
        return count;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            continue;
        }
        if ft.is_dir() {
            count = count.saturating_add(walk_and_count(root, &path, hits));
        } else if !ft.is_dir() && path.extension().is_some_and(|e| e == "rs") {
            count = count.saturating_add(1);
            scan_file(root, &path, hits);
        }
    }
    count
}

fn scan_file(root: &Path, path: &Path, hits: &mut HashMap<String, TodoHit>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    // Multi-line regex on full content; line number computed from match start byte offset.
    let Ok(regex) = todo_regex() else { return };
    for caps in regex.captures_iter(&content) {
        let Some(m) = caps.get(0) else { continue };
        let Some(desc) = caps.get(1) else { continue };
        let line_no = content
            .get(..m.start())
            .map_or(0, |s| s.matches('\n').count())
            .saturating_add(1);
        let hit = TodoHit {
            file: rel.clone(),
            line: line_no,
            description: desc.as_str().to_owned(),
        };
        hits.insert(key_for(&hit), hit);
    }
}
