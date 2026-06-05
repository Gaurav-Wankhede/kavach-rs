//! Verify completion claims before allowing "done" declarations.
//!
//! Detects premature completion claims by checking if the agent has
//! actually run tests, verified builds, or checked results before
//! claiming work is finished.

use kavach_session::SessionState;

/// Phrases that indicate premature completion claims.
const PREMATURE_DONE_PHRASES: &[&str] = &[
    "all tests pass",
    "everything works",
    "implementation is complete",
    "all done",
    "task completed",
    "successfully implemented",
    "fix is in place",
    "changes are ready",
    "ready for review",
    "should work now",
    "that should fix",
];

/// Check if content claims completion without evidence.
/// Returns Some(warning) if premature completion detected.
pub(crate) fn check_completion_claim(content: &str, session: &SessionState) -> Option<String> {
    let lower = content.to_lowercase();

    let has_claim = PREMATURE_DONE_PHRASES
        .iter()
        .any(|phrase| lower.contains(phrase));

    if !has_claim {
        return None;
    }

    if has_verification_evidence(session) {
        return None;
    }

    Some(
        "[COMPLETION_CHECK]\n\
         Completion claimed but no verification evidence found.\n\
         Run `cargo test` and `cargo check`, then show the actual output as evidence.\n\
         Report completion only after verified output confirms success."
            .into(),
    )
}

/// Check session for evidence of recent verification commands.
fn has_verification_evidence(session: &SessionState) -> bool {
    session.recent_commands.iter().any(|cmd| {
        let c = cmd.to_lowercase();
        c.contains("cargo test")
            || c.contains("cargo check")
            || c.contains("cargo clippy")
            || c.contains("cargo build")
            || c.contains("npm test")
            || c.contains("pytest")
            || c.contains("go test")
    })
}

// NOTE: `check_review_isolation` was REMOVED under the "kill blocking, keep
// auto-continue" policy — it was a Stop HALT nag (block on completion-language +
// 5+ modified files + no review). The Stop gate no longer halts; only
// `check_completion_claim` remains, consumed at PreWrite time by
// `post_write_checks.rs` as a non-Stop advisory.

#[cfg(test)]
#[path = "completion_guard_tests.rs"]
mod tests;
