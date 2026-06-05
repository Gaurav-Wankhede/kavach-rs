//! Detect duplicate tool calls that waste tokens.
//!
//! Tracks recent Read/Glob/Grep/WebSearch calls. Warns if the same
//! file is read or the same search is executed multiple times without
//! changes in between.

use kavach_session::SessionState;

const MAX_DUPLICATES: usize = 2;

/// Check if a read/search tool call is a duplicate.
/// Returns Some(warning) if duplicate detected.
pub(crate) fn check_duplicate_tool(
    session: &SessionState,
    tool_name: &str,
    target: &str,
) -> Option<String> {
    if target.is_empty() {
        return None;
    }

    let key = format!("{tool_name}:{target}");
    let count = session
        .recent_commands
        .iter()
        .filter(|c| c.as_str() == key)
        .count();

    if count >= MAX_DUPLICATES {
        return Some(format!(
            "[DUPLICATE_TOOL]\n\
             {tool_name} called {count}x on: {}\n\
             Use the data you already have.\n\
             1) Check git diff if the file may have changed.\n\
             2) Refine search patterns instead of repeating.\n\
             3) Move forward with existing results.",
            truncate(target, 60)
        ));
    }
    None
}

/// Record a tool call for duplicate tracking.
pub(crate) fn record_tool_call(session: &mut SessionState, tool_name: &str, target: &str) {
    if target.is_empty() {
        return;
    }
    let key = format!("{tool_name}:{target}");
    session.recent_commands.push(key);
    // Shared history with loop_guard — cap at 30 total
    if session.recent_commands.len() > 30 {
        session.recent_commands.remove(0);
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        s.char_indices()
            .nth(max)
            .and_then(|(idx, _)| s.get(..idx))
            .unwrap_or(s)
    }
}

#[cfg(test)]
#[path = "duplicate_tool_guard_tests.rs"]
mod tests;
