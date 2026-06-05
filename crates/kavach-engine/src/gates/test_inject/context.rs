//! Build the `[TEST_ENFORCEMENT]` context injected into intent-gate output.
use kavach_session::SessionState;

use super::action::{nudge_level, scoped_action};

/// Build test enforcement context to inject into intent gate output.
/// Returns None if no test debt, Some(context) if escalation is needed.
pub(crate) fn build_test_context(session: &mut SessionState) -> Option<String> {
    if !session.has_pending_tests() {
        return None;
    }
    let count = session.test_files_pending.len();
    let level = nudge_level(session.test_nudge_count);
    session.test_nudge_count = session.test_nudge_count.saturating_add(1);
    session.save().ok();

    let files = session.test_files_pending.join(", ");
    let action = scoped_action(&session.test_files_pending);
    Some(format!(
        "[TEST_ENFORCEMENT]\nstatus: {level}\npending: {count} file(s)\nfiles: {files}\naction: {action}\n"
    ))
}
