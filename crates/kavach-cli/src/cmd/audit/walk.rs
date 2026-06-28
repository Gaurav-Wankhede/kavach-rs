//! ONE source-file walker for the consolidated audit — adopts the loophole
//! sweep's gold-standard test-exclusion (best of the four pre-merge walkers).
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "dist", "build", ".venv"];

/// Collect scannable `.rs` sources under `root`, skipping VCS/build/vendor dirs
/// and every test-file convention.
pub(super) fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                collect(&path, out);
            }
        } else if is_scannable_rust(&path.to_string_lossy()) {
            out.push(path);
        }
    }
}

/// True for a non-test Rust source. Excludes `tests.rs`, any `_test` stem
/// segment, and anything under a `tests/` dir.
pub(super) fn is_scannable_rust(path: &str) -> bool {
    let p = Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rs")) {
        return false;
    }
    let s = path.replace('\\', "/");
    if s.contains("/tests/") || s.starts_with("tests/") {
        return false;
    }
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name != "tests.rs" && !name.trim_end_matches(".rs").contains("_test"))
}

#[cfg(test)]
#[path = "walk_test.rs"]
mod walk_test;
