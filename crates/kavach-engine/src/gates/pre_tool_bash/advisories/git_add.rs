//! Blanket-staging detector: `git add .`, `git add -A`, `git add --all`.

/// Detect `git add .`, `git add -A`, `git add --all` — blanket staging.
pub(in crate::gates::pre_tool_bash) fn is_git_add_all(cmd: &str) -> bool {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.len() < 3 {
        return false;
    }
    if parts.first().is_some_and(|p| *p != "git") {
        return false;
    }
    if parts.get(1).is_some_and(|p| *p != "add") {
        return false;
    }
    let arg = parts.get(2).copied().unwrap_or("");
    arg == "." || arg == "-A" || arg == "--all"
}
