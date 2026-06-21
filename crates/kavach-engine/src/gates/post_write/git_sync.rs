//! Post-write git-sync ADVISORY stage (read-only; never mutates the repo).
//!
//! Emits a `[GIT_SYNC]` block after a Write/Edit so the agent stays current with git:
//! branch · ahead/behind upstream · uncommitted count · suggested commit · conflict
//! warning for the just-written file · open-PR review status with a `/pr-review` pointer.
//!
//! DECISION (2026-06-21): advisory-only. The hook NEVER runs `git commit/push` or touches
//! GitHub — auto-push is outward-facing and hard to reverse. PR reviews route to the
//! `/pr-review` command, which the agent runs. Every probe fails OPEN: no advisory line
//! on any git/gh error, so a VCS hiccup can never block the post-write pipeline.

mod conflict;
mod git;
mod pr;

/// Build the `[GIT_SYNC]` advisory for a just-written file, or `None` when nothing
/// actionable is pending (clean tree, no PR, no conflict). Read-only throughout.
pub(super) fn advisory(file_path: &str, content: &str) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();

    if let Some(g) = git::probe() {
        lines.push(format!("branch: {}", g.branch));
        if g.ahead > 0 || g.behind > 0 {
            lines.push(format!("upstream: ahead {} · behind {}", g.ahead, g.behind));
        }
        if g.uncommitted > 0 {
            lines.push(format!("uncommitted: {} file(s) — not yet committed", g.uncommitted));
            lines.push(format!("RUN now: git add -A && git commit -m {:?}", commit_msg(file_path)));
        }
        if g.behind > 0 {
            lines.push("behind upstream — `git pull --rebase` before pushing to avoid a merge".into());
        }
    }

    if conflict::has_conflict_markers(content) {
        lines.push(format!("CONFLICT: unresolved merge markers in {file_path} — resolve before committing"));
    }

    if let Some(p) = pr::probe() {
        lines.push(pr_line(&p));
        lines.push("run `/pr-review` to triage review threads on this PR".into());
    }

    if lines.is_empty() {
        return None;
    }
    Some(format!("[GIT_SYNC]\n{}", lines.join("\n")))
}

/// Derive a Conventional-Commit-shaped suggestion from the written file's basename.
fn commit_msg(file_path: &str) -> String {
    let name = file_path.rsplit('/').next().unwrap_or(file_path);
    format!("chore: update {name}")
}

/// One-line PR status, steering to `/pr-review` when human review is outstanding.
fn pr_line(p: &pr::PrState) -> String {
    let status = match p.decision.as_str() {
        "CHANGES_REQUESTED" => "changes requested",
        "APPROVED" => "approved",
        "REVIEW_REQUIRED" => "review required",
        _ => "open",
    };
    format!("PR #{}: {status}", p.number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_msg_uses_basename() {
        assert_eq!(commit_msg("crates/foo/src/bar.rs"), "chore: update bar.rs");
        assert_eq!(commit_msg("bare.rs"), "chore: update bare.rs");
    }

    #[test]
    fn conflict_markers_detected_only_at_line_start() {
        assert!(conflict::has_conflict_markers("a\n<<<<<<< HEAD\nb\n======="));
        assert!(conflict::has_conflict_markers(">>>>>>> branch\n"));
        // A docstring mentioning the markers in prose must NOT trip the scan.
        assert!(!conflict::has_conflict_markers("// uses <<<<<<< for diffs\nfn x() {}"));
        assert!(!conflict::has_conflict_markers("clean source\n"));
    }

    #[test]
    fn pr_line_steers_to_review_when_changes_requested() {
        let p = pr::PrState { number: 42, decision: "CHANGES_REQUESTED".into() };
        assert_eq!(pr_line(&p), "PR #42: changes requested");
    }
}
