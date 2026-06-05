//! Sliding-window repeat detection over the recent-command history.
use super::history::{normalize_command, parse_entry, truncate};
use super::inspection::{git_diff_has_pending_changes, is_inspection_command};
use kavach_session::SessionState;

const MAX_REPEATS: usize = 3;
/// Only count repeats within the last N turns (sliding window).
pub(super) const WINDOW_TURNS: i32 = 10;

/// Check if a bash command is being repeated too many times within the sliding
/// window. Returns `Some(block_reason)` when a loop is detected.
///
/// FIX: `silent_failure` — pure-inspection commands (`wc`/`stat`/…) re-run 3x to
/// verify file size after each Edit produce distinct results, but text-equality
/// detection cannot model the intervening file mutation. When
/// `KAVACH_NEW_DETECTORS=1`, inspection commands are exempt IF `git diff --stat`
/// shows pending source changes (a proxy for "file mutated between calls").
/// Legacy behavior is preserved when the env var is unset.
/// RESEARCH: <https://github.com/anthropics/claude-code/issues/12667> (stop hook
/// misfire UX); <https://proofademic.ai/blog/false-positives-ai-detection-guide/>.
pub(crate) fn check_bash_loop(session: &SessionState, command: &str) -> Option<String> {
    let normalized = normalize_command(command);
    let window_start = session.turn_count.saturating_sub(WINDOW_TURNS);

    let count = session
        .recent_commands
        .iter()
        .filter(|entry| {
            let (turn, cmd) = parse_entry(entry);
            turn >= window_start && normalize_command(cmd) == normalized
        })
        .count();

    if count < MAX_REPEATS {
        return None;
    }
    if std::env::var("KAVACH_NEW_DETECTORS").as_deref() == Ok("1")
        && is_inspection_command(&normalized)
        && git_diff_has_pending_changes()
    {
        // File mutated between identical inspection calls; re-running is
        // legitimate verification, not a retry loop.
        return None;
    }
    Some(format!(
        "[LOOP_DETECTED]\n\
         Command executed {count}x in last {WINDOW_TURNS} turns: `{}`\n\
         BLOCKED: Repeating the same command won't produce different results.\n\
         FIX: 1) Diagnose WHY it fails, don't just retry.\n\
         2) Try a different approach or tool.\n\
         3) Use /clear if context is polluted with failed attempts.",
        truncate(command, 80)
    ))
}
