//! Pure-inspection command classification + git-pending-changes proxy.
//!
//! Inspection commands (read-only, idempotent on static state) legitimately
//! re-run after intervening Edits to verify mutated state, so they are exempt
//! from the loop block when the source tree shows pending changes.

/// Is this a pure-inspection command (read-only, idempotent on static state)?
/// Matches first-token only; pipelines that BEGIN with these utilities qualify
/// (e.g. `wc -c file && echo done`).
pub(super) fn is_inspection_command(normalized: &str) -> bool {
    // Legacy POSIX tools + Rust toolbelt replacements.
    // SOURCE: kavach::toolbelt::Tool enum + brahmastra-boot.sh installer set.
    // RCA: decision:u7wxd7ykt3va28ai99or (INSPECTION_BINS drift fix).
    const INSPECTION_BINS: &[&str] = &[
        // POSIX legacy
        "wc",
        "stat",
        "ls",
        "cat",
        "file",
        "du",
        "head",
        "tail",
        "ps",
        "tree",
        "grep",
        "find",
        "diff",
        "jq", // Rust toolbelt (mirrors kavach::toolbelt::Tool)
        "rg",
        "fd",
        "bat",
        "eza",
        "erd",
        "procs",
        "dust",
        "tokei",
        "hyperfine",
        "jaq",
        "gron",
        "dasel",
        "sg",
        "difft",
    ];
    // Defense-in-depth: reject output redirection (writes to file).
    if normalized.contains('>') {
        return false;
    }
    // Reject pipes into known mutators (tee, xargs to kill/rm, sudo).
    let mutator_pipes = ["| tee", "|tee", "| xargs", "|xargs", "| sudo", "|sudo"];
    if mutator_pipes.iter().any(|m| normalized.contains(m)) {
        return false;
    }
    // For pipelines, ALL stages must be inspection-only (e.g. `bat file | head`).
    if normalized.contains('|') {
        return normalized.split('|').all(|stage| {
            let first = stage.split_whitespace().next().unwrap_or("");
            let bin = first.rsplit('/').next().unwrap_or(first);
            INSPECTION_BINS.contains(&bin)
        });
    }
    let first = normalized.split_whitespace().next().unwrap_or("");
    // Strip path prefix (e.g. /usr/bin/wc → wc)
    let bin = first.rsplit('/').next().unwrap_or(first);
    INSPECTION_BINS.contains(&bin)
}

/// Does `git diff --stat` show any pending source-tree changes? Proxy for
/// "filesystem mutated since prior identical inspection command". Fail-closed:
/// if git is unavailable, returns false (preserves the legacy block).
pub(super) fn git_diff_has_pending_changes() -> bool {
    crate::toolbelt::git_has_pending_changes()
}
