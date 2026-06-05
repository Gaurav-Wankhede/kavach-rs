// kavach tailwind-plus index — walk ~/.claude/tailwind-plus/, extract keywords, write index.json
// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
// ALGO: iterative DFS via explicit Vec stack (preserved verbatim; not modified by this silent-IO migration). TIME: O(N) over file tree. SOURCE: https://doc.rust-lang.org/std/collections/struct.VecDeque.html
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use kavach_config::{tailwind_plus_dir, tailwind_plus_index};
use serde_json::json;

use crate::cli::TailwindPlusAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Dispatch `kavach tailwind-plus <action>`.
pub(super) fn run(action: TailwindPlusAction) -> i32 {
    match action {
        TailwindPlusAction::Index => handle_index(),
    }
}

fn handle_index() -> i32 {
    let base = tailwind_plus_dir();
    if !base.exists() {
        let msg = format!("tailwind-plus dir not found: {}", base.display());
        if let Err(e) = ewrite_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 1;
    }
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![base.clone()];
    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => {
                let msg = format!("read_dir {} failed: {e}", dir.display());
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        for entry_result in read_dir {
            let Ok(entry) = entry_result else { continue };
            let path = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(path);
            } else if !ft.is_dir()
                && is_component_file(&path)
                && let Some(record) = build_record(&base, &path)
            {
                entries.push(record);
            }
        }
    }
    let index = json!({ "components": entries });
    let json_str = match serde_json::to_string_pretty(&index) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("serialize failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let index_path = tailwind_plus_index();
    if let Some(parent) = index_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        let msg = format!("create dir failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    if let Err(e) = fs::write(&index_path, &json_str) {
        let msg = format!("write index failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let msg = format!(
        "indexed {} components → {}",
        entries.len(),
        index_path.display()
    );
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

fn is_component_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jsx" | "tsx" | "js" | "ts") => true,
        Some(_) | None => false,
    }
}

/// Build a single index record for one component file.
fn build_record(base: &Path, path: &Path) -> Option<serde_json::Value> {
    let Ok(rel) = path.strip_prefix(base) else {
        return None;
    };
    let rel_str = match rel.to_str() {
        Some(s) => s.replace('\\', "/"),
        None => return None,
    };
    let category = rel
        .parent()
        .and_then(|p| p.to_str())
        .map_or_else(String::new, |s| s.replace('\\', "/"));
    let name = match path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n.to_owned(),
        None => return None,
    };
    let keywords = extract_keywords(&category, &name, path);
    let lines = count_lines(path);
    Some(json!({
        "category": category,
        "name": name,
        "file": rel_str,
        "keywords": keywords,
        "lines": lines,
    }))
}

/// Keywords from: category path segments + file stem + first-5-line tokens.
fn extract_keywords(category: &str, name: &str, path: &Path) -> Vec<String> {
    let mut kw: Vec<String> = Vec::new();
    for seg in category.split('/') {
        let seg = seg.trim();
        if !seg.is_empty() {
            for part in seg.split('-') {
                let part = part.trim();
                if part.len() > 1 {
                    kw.push(part.to_lowercase());
                }
            }
        }
    }
    for part in name.split('-') {
        let part = part.trim();
        if part.len() > 1 {
            kw.push(part.to_lowercase());
        }
    }
    if let Ok(body) = fs::read_to_string(path) {
        for line in body.lines().take(5) {
            for token in line.split(|c: char| !c.is_alphanumeric() && c != '-') {
                let t = token.trim().to_lowercase();
                if t.len() > 2 && !is_noise_token(&t) {
                    kw.push(t);
                }
            }
        }
    }
    let mut seen = HashSet::new();
    kw.retain(|k| seen.insert(k.clone()));
    kw
}

fn is_noise_token(t: &str) -> bool {
    matches!(
        t,
        "import"
            | "from"
            | "use"
            | "const"
            | "let"
            | "var"
            | "export"
            | "default"
            | "function"
            | "return"
            | "react"
            | "jsx"
            | "tsx"
    )
}

fn count_lines(path: &Path) -> usize {
    fs::read_to_string(path).map_or(0, |s| s.lines().count())
}
