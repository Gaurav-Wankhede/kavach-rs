//! Path classification for the six-file path gate: an allowlist of universal
//! project docs + a regex matching the forbidden markdown spec/context files.
use std::path::Path;
use std::sync::LazyLock;

static FORBIDDEN_REGEX: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(
        r"(^|/)(context|docs)/(project-overview|architecture|code-standards|ai-workflow-rules|ui-context|progress-tracker)\.md$|(^|/)(PROJECT-OVERVIEW|ARCHITECTURE|CODE-STANDARDS|AI-WORKFLOW-RULES|UI-CONTEXT|PROGRESS-TRACKER|OVERVIEW)\.md$|(^|/)specs?/[0-9]+[-_].*\.md$|\.spec\.md$|(^|/)progress-tracker\.md$",
    )
    .ok()
});

/// Universal project docs that are always permitted (never six-file context).
pub(super) fn is_allowlisted(path: &str) -> bool {
    let basename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    matches!(
        basename,
        "CLAUDE.md" | "AGENTS.md" | "README.md" | "CHANGELOG.md" | "CONTRIBUTING.md" | "LICENSE.md"
    )
}

/// True when `path` is a markdown spec/context file that duplicates kavach-db state.
pub(super) fn is_forbidden(path: &str) -> bool {
    FORBIDDEN_REGEX.as_ref().is_some_and(|re| re.is_match(path))
}
