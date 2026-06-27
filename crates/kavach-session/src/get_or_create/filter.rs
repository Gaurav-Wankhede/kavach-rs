//! Cross-project leakage guard for `test_files_pending`.

use crate::state::SessionState;

/// Remove `test_files_pending` entries that don't belong to the current `work_dir`.
/// Prevents cross-project test enforcement leakage when switching directories.
pub(crate) fn filter_test_pending_for_project(state: &mut SessionState, work_dir: &str) {
    if work_dir.is_empty() || state.test_files_pending.is_empty() {
        return;
    }
    let before = state.test_files_pending.len();
    // Ensure trailing separator to prevent "/kavach" matching "/kavach-backup"
    let prefix = if work_dir.ends_with('/') {
        work_dir.to_owned()
    } else {
        format!("{work_dir}/")
    };
    state.test_files_pending.retain(|f| f.starts_with(&prefix));
    if state.test_files_pending.is_empty() && before > 0 {
        state.test_nudge_count = 0;
    }
}

#[cfg(test)]
#[path = "filter_test.rs"]
#[cfg(test)]
#[path = "filter_test.rs"]
mod tests;