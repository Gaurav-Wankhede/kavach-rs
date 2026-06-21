//! Repo-wide micro-file linter: walks `.rs` files under the given roots and runs
//! the SAME `micro_file_guard::detect()` the write-time gate uses, so a human
//! commit or a legacy file is held to the identical rule. Exits 1 on any
//! violation — wire into CI / pre-commit. Usage: `micro-file-lint <path>...`

// This is a CLI reporting tool whose entire job is stdout/stderr output, so
// print macros are correct here (not the library no-print rule). The counter
// increment is bounded by the file count and cannot overflow in practice.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "CLI linter: stdout/stderr IS the deliverable"
)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "violation counter bounded by scanned-file count; no realistic overflow"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    if roots.is_empty() {
        eprintln!("usage: micro-file-lint <path>...  (scans *.rs recursively)");
        return ExitCode::FAILURE;
    }

    let mut files = Vec::new();
    for root in &roots {
        collect_rs(std::path::Path::new(root), &mut files);
    }

    let mut violations = 0_usize;
    for path in &files {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let path_str = path.to_string_lossy();
        // tool_name "Edit" => existing-file LOC message; this is a repo scan of
        // files already on disk, never a fresh Write.
        for v in kavach_patterns::micro_file_guard::detect(&path_str, &content, "Edit") {
            violations += 1;
            println!("{path_str}: [{}] {}", v.pattern, v.fix);
        }
    }

    if violations == 0 {
        println!("micro-file-lint: clean ({} files scanned)", files.len());
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "micro-file-lint: {violations} violation(s) across {} files",
            files.len()
        );
        ExitCode::FAILURE
    }
}

// recursion — bounded stack memory, no overflow on deep monorepos).
//   - recursive DFS: clean but risks stack overflow on pathological depth; the
//     micro-file rule itself caps src depth at 7 but vendor/target dirs can be deeper.
//   - BFS (VecDeque): same O(n) work, worse locality, no ordering benefit here.
//   - `walkdir`/`ignore` crates: heavier dep for a CI helper; std read_dir suffices.
// TIME: O(n) over n filesystem entries. SPACE: O(d) stack, d = max live dir fan-out.
// YEAR: 2026 | SEARCHED: 2026-06
fn collect_rs(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    // A root may be a single file (CI / pre-commit pass changed-file paths) or a
    // directory (full-tree scan). Take a file path directly; only walk dirs.
    if root.is_file() {
        if root.extension().is_some_and(|e| e == "rs") {
            out.push(root.to_path_buf());
        }
        return;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
}
