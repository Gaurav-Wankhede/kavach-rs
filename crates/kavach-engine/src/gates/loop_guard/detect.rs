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
        return None;
    }
    Some(format!(
        "[LOOP_LIMIT] Command executed {count}x in last {WINDOW_TURNS} turns: `{}` \
         — repeating it won't produce different results \
         -> diagnose WHY it fails (don't just retry), try a different approach or tool, \
         or use /clear if context is polluted with failed attempts -> then retry.",
        truncate(command, 80)
    ))
}

const TOOL_HISTORY_SIZE: usize = 30;

pub(crate) fn check_tool_loop(session: &SessionState, tool_name: &str, tool_input: &str) -> Option<String> {
    let normalized_input = normalize_command(tool_input);
    let key = format!("{}:{}", tool_name, normalized_input);
    let window_start = session.turn_count.saturating_sub(WINDOW_TURNS);

    let count = session
        .recent_tool_calls
        .iter()
        .filter(|entry| {
            let (turn, recorded) = parse_entry(entry);
            turn >= window_start && recorded == key
        })
        .count();

    if count < MAX_REPEATS {
        return None;
    }
    Some(format!(
        "[LOOP_LIMIT] STOP: Same tool call executed {count}x in last {WINDOW_TURNS} turns: `{tool_name}` \
         — repeating it won't produce different results. \
         DIAGNOSE ROOT CAUSE BEFORE RETRYING: \
         (1) Is the argument correct? \
         (2) Is the file path right? \
         (3) Read the error message carefully. \
         (4) Try a fundamentally different approach or tool. \
         (5) Use /clear if context is polluted with failed attempts."
    ))
}

pub(crate) fn record_tool_call(session: &mut SessionState, tool_name: &str, tool_input: &str) {
    let normalized_input = normalize_command(tool_input);
    let entry = format!("{}:{}:{}", session.turn_count, tool_name, normalized_input);
    session.recent_tool_calls.push(entry);
    if session.recent_tool_calls.len() > TOOL_HISTORY_SIZE {
        session.recent_tool_calls.rotate_left(1);
        session.recent_tool_calls.pop();
    }
}
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
        "[LOOP_LIMIT] Command executed {count}x in last {WINDOW_TURNS} turns: `{}` \
         — repeating it won't produce different results \
         -> diagnose WHY it fails (don't just retry), try a different approach or tool, \
         or use /clear if context is polluted with failed attempts -> then retry.",
        truncate(command, 80)
    ))
}
