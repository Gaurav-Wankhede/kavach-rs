//! RCA-gate applicability rules: which intents/risks require an `[RCA]` block,
//! and which file paths are exempt per `CLAUDE.md` §RCA EXEMPT.

/// Intents that REQUIRE an `[RCA]` block before `Edit`/`Write`.
/// Matches `CLAUDE.md` §4 — "every debug/refactor/implement intent".
pub(super) const REQUIRES_RCA: &[&str] = &["debug", "refactor", "implement"];

/// Risk levels that REQUIRE an `[RCA]` block.
/// `low` is exempt — trivial typo/rename/format changes per §6 carve-out.
pub(super) fn risk_requires_rca(risk: &str) -> bool {
    matches!(risk, "medium" | "high" | "critical")
}

/// File path patterns exempt from the RCA gate per `CLAUDE.md` §RCA EXEMPT list.
///
/// - `~/.claude/*.md` config files
/// - `*.json` settings files
/// - `CLAUDE.md` project instructions
pub(super) fn is_rca_exempt_path(file_path: &str) -> bool {
    const SAFE_JSON_CONFIGS: &[&str] = &[
        "settings.json",
        "settings.local.json",
        ".vscode/settings.json",
        "keybindings.json",
    ];
    if file_path.is_empty() {
        return false;
    }
    if file_path.contains("/.claude/")
        && std::path::Path::new(file_path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    {
        return true;
    }
    if SAFE_JSON_CONFIGS.iter().any(|c| file_path.ends_with(c)) {
        return true;
    }
    // Project-root CLAUDE.md and global ~/.claude/CLAUDE.md only.
    // A nested src/.../CLAUDE.md is treated as a normal file and stays gated.
    file_path == "CLAUDE.md" || file_path.ends_with("/.claude/CLAUDE.md")
}
