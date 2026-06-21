//! Read-only git state probe (branch, ahead/behind, uncommitted count).
//!
//! NEVER mutates the repo — advisory-only per the git-sync decision. Every probe
//! fails OPEN: a git error yields `None` so the post-write pipeline is never blocked.
//! SOURCE: <https://git-scm.com/docs/git-status#_porcelain_format_version_1> (`-b` header).

use std::process::Command;

/// A read-only snapshot of working-tree git state.
pub(super) struct GitState {
    /// Current branch (`HEAD` when detached).
    pub branch: String,
    /// Commits ahead of upstream (0 when no upstream / unknown).
    pub ahead: u32,
    /// Commits behind upstream (0 when no upstream / unknown).
    pub behind: u32,
    /// Count of files with staged or unstaged changes (untracked included).
    pub uncommitted: usize,
}

/// Probe git via `status --porcelain=v1 -b`. `None` outside a repo or on any error.
pub(super) fn probe() -> Option<GitState> {
    let out = Command::new("git")
        .args(["status", "--porcelain=v1", "-b", "--untracked-files=normal"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    let uncommitted = lines.filter(|l| !l.is_empty()).count();
    let (branch, ahead, behind) = parse_branch_header(header);
    Some(GitState { branch, ahead, behind, uncommitted })
}

/// Parse the `## branch...upstream [ahead N, behind M]` porcelain header line.
fn parse_branch_header(header: &str) -> (String, u32, u32) {
    let body = header.strip_prefix("## ").unwrap_or(header);
    let branch_part = body.split("...").next().unwrap_or(body);
    let branch = branch_part.split_whitespace().next().unwrap_or("HEAD").to_owned();
    let ahead = extract_track(body, "ahead ");
    let behind = extract_track(body, "behind ");
    (branch, ahead, behind)
}

/// Pull the integer following a `marker` (`ahead `/`behind `) inside the `[...]` block.
/// Char-safe: splits on the marker and reads ASCII digits, never byte-indexing.
fn extract_track(body: &str, marker: &str) -> u32 {
    body.split_once(marker)
        .and_then(|(_, rest)| {
            let digits: String = rest.trim_start().chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .unwrap_or(0)
}
