//! Auto-allow classifiers: read-only tools, kavach CLI, and safe cache/build rm.

/// Read-only / side-effect-free tools that never need a permission prompt.
pub(super) fn is_safe_auto_allow(tool: &str) -> bool {
    matches!(
        tool,
        "Read"
            | "Grep"
            | "Glob"
            | "WebSearch"
            | "WebFetch"
            | "Skill"
            | "ToolSearch"
            | "AskUserQuestion"
    )
}

/// True iff `cmd` invokes the kavach CLI (auto-allowed harness tooling).
pub(super) fn is_kavach_command(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    trimmed.starts_with("kavach ") || trimmed == "kavach"
}

/// Auto-allow `rm` targeting known safe cache/build directories.
/// Checks each rm argument's final path component (basename) to avoid false
/// matches on "build"/"dist" as substrings of unrelated paths.
pub(super) fn is_safe_rm_target(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    if !lower.contains("rm ") {
        return false;
    }
    let safe_names = [
        "node_modules",
        ".vite",
        ".next",
        ".astro",
        ".cache",
        ".turbo",
        ".parcel-cache",
        ".output",
        "__pycache__",
        "dist",
        "build",
    ];
    let safe_paths = ["target/debug", "target/release"];
    let parts: Vec<&str> = lower.split_whitespace().collect();
    let past_rm = parts.iter().skip_while(|p| !p.starts_with("rm")).skip(1);
    for arg in past_rm {
        if arg.starts_with('-') {
            continue;
        }
        let path = arg.trim_matches(|c: char| c == '"' || c == '\'');
        if safe_paths.iter().any(|s| path.ends_with(s)) {
            return true;
        }
        let basename = path.rsplit('/').next().unwrap_or(path);
        if safe_names.contains(&basename) {
            return true;
        }
    }
    false
}
