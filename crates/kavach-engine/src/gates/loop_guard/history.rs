//! Command-history ring buffer + entry parsing/normalization helpers.
//!
//! Storage format: `"<turn>:<command>"` — turn prefix encoded inline in the
//! existing `Vec<String>` field so serialization is unchanged. Legacy entries
//! without a turn prefix are assigned turn 0 and age out of the window.
use kavach_session::SessionState;

pub(super) const HISTORY_SIZE: usize = 20;

/// Record a command in the session's recent history as `"<turn>:<command>"`.
/// Bounded ring: evicts the oldest entry when `HISTORY_SIZE` is exceeded.
pub(crate) fn record_command(session: &mut SessionState, command: &str) {
    let entry = format!("{}:{}", session.turn_count, command);
    session.recent_commands.push(entry);
    if session.recent_commands.len() > HISTORY_SIZE {
        // Remove index 0 without shifting: rotate the oldest to the end, then
        // pop. Vec::remove(0) is O(N); this is O(1) amortized, order-preserving.
        session.recent_commands.rotate_left(1);
        session.recent_commands.pop();
    }
}

/// Parse a history entry into (turn, `command_str`). Format: `"<turn>:<command>"`.
/// Legacy bare entries (no colon-prefixed integer) return turn 0.
pub(super) fn parse_entry(entry: &str) -> (i32, &str) {
    if let Some(colon) = entry.find(':')
        && let Some(turn_str) = entry.get(..colon)
        && let Ok(turn) = turn_str.parse::<i32>()
    {
        let cmd = entry.get(colon.saturating_add(1)..).unwrap_or("");
        return (turn, cmd);
    }
    (0, entry)
}

/// Normalize a command for comparison: trim + collapse runs of whitespace.
pub(super) fn normalize_command(cmd: &str) -> String {
    cmd.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Borrow the leading `max` bytes of `s` (whole string when shorter).
pub(super) fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        s.get(..max).unwrap_or(s)
    }
}
