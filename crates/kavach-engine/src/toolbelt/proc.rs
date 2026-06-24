//! Process + git inspection wrappers (procs, git diff).
use std::process::Command;

use super::tool::Tool;

/// Run `git diff --stat` and return parsed output.
///
/// Returns Ok((true, stdout)) when git succeeds and there are changes,
/// Ok((false, "")) when there are no changes,
/// Err(_) when git is unavailable or the command fails.
/// Used by gates to detect filesystem mutation between turns.
/// SOURCE: shelling out preserves bit-exact behavior with user's git config (vs git2).
///
/// # Errors
/// Returns `Err` when git is unavailable or the command fails.
pub fn git_diff_stat() -> Result<(bool, String), String> {
    let output = Command::new("git")
        .args(["diff", "--stat"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("git diff failed: exit {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok((!stdout.is_empty(), stdout))
}

/// Run `git diff --stat` and return whether any files were modified.
/// Fail-closed: returns false on git failure (preserves legacy gate behavior).
#[must_use]
pub fn git_has_pending_changes() -> bool {
    matches!(git_diff_stat(), Ok((true, _)))
}
