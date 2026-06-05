use crate::load::{load_session_state, load_session_state_for};
use crate::paths::detect_project;
use crate::state::SessionState;

/// Load existing session or create a new one (no `session_id` context).
///
/// Prefer `get_or_create_session_for` when a `session_id` is available — it
/// resolves the durable DB row and is immune to cross-`/clear` rehydration.
#[must_use]
pub fn get_or_create_session() -> SessionState {
    materialize(load_session_state().ok().flatten())
}

/// Session-aware load: resolve the durable `session_runtime` DB row.
///
/// A `/clear` (new `session_id`) gets a fresh state instead of rehydrating
/// the prior conversation's INI file. Falls back to the file-only path when
/// `session_id` is empty.
#[must_use]
pub fn get_or_create_session_for(session_id: &str) -> SessionState {
    let loaded = if session_id.is_empty() {
        load_session_state().ok().flatten()
    } else {
        load_session_state_for(session_id)
    };
    let mut state = materialize(loaded);
    if !session_id.is_empty() {
        session_id.clone_into(&mut state.session_id);
    }
    state
}

/// Resolve the session id from the `KAVACH_SESSION_ID` env (Claude Code sets it
/// on every hook). Empty if unset — callers guard on emptiness.
fn env_session_id() -> String {
    std::env::var("KAVACH_SESSION_ID").unwrap_or_default()
}

/// Apply `work_dir` / project refresh to a loaded state, or build a fresh one.
fn materialize(loaded: Option<SessionState>) -> SessionState {
    let wd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    if let Some(mut state) = loaded {
        let old_wd = state.work_dir.clone();
        wd.clone_into(&mut state.work_dir);
        state.project = detect_project();
        // A loaded state can predate session_start (or come from a stale INI)
        // and carry an empty session_id. Backfill from the env so EVERY gate —
        // not just those running after a save — sees the real id (trajectory
        // capture, mistake ledger, durable resume all key off it).
        if state.session_id.is_empty() {
            state.session_id = env_session_id();
        }
        if old_wd != wd {
            filter_test_pending_for_project(&mut state, &wd);
        }
        // Surface save failures on stderr instead of silently dropping them.
        // A failing save means the next hook will see stale state — operators
        // need this signal in the audit trail.
        #[expect(clippy::print_stderr, reason = "diagnostic output to audit trail")]
        if let Err(e) = state.save() {
            eprintln!("[session] materialize: save failed (state may be stale): {e}");
        }
        state
    } else {
        let mut state = SessionState::new(&wd);
        state.session_id = env_session_id();
        #[expect(clippy::print_stderr, reason = "diagnostic output to audit trail")]
        if let Err(e) = state.save() {
            eprintln!("[session] materialize: initial save failed: {e}");
        }
        state
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn filter_removes_other_project_files() {
        let mut s = SessionState::default();
        s.test_files_pending = vec![
            "/Users/g/Astro/astro-advisor/src/forecast.rs".into(),
            "/Users/g/Nicole/Backend/src/routes.rs".into(),
        ];
        s.test_nudge_count = 5;
        filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
        assert_eq!(s.test_files_pending.len(), 1);
        assert_eq!(
            s.test_files_pending[0],
            "/Users/g/Nicole/Backend/src/routes.rs"
        );
        assert_eq!(s.test_nudge_count, 5);
    }

    #[test]
    fn filter_resets_nudge_when_all_cleared() {
        let mut s = SessionState::default();
        s.test_files_pending = vec!["/Users/g/Astro/src/forecast.rs".into()];
        s.test_nudge_count = 49;
        filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
        assert!(s.test_files_pending.is_empty());
        assert_eq!(s.test_nudge_count, 0);
    }

    #[test]
    fn filter_keeps_all_when_same_project() {
        let mut s = SessionState::default();
        s.test_files_pending = vec![
            "/Users/g/Nicole/src/auth.rs".into(),
            "/Users/g/Nicole/src/pay.rs".into(),
        ];
        s.test_nudge_count = 3;
        filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
        assert_eq!(s.test_files_pending.len(), 2);
        assert_eq!(s.test_nudge_count, 3);
    }

    #[test]
    fn filter_noop_on_empty_pending() {
        let mut s = SessionState::default();
        s.test_nudge_count = 2;
        filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
        assert!(s.test_files_pending.is_empty());
        assert_eq!(s.test_nudge_count, 2);
    }

    #[test]
    fn filter_noop_on_empty_work_dir() {
        let mut s = SessionState::default();
        s.test_files_pending = vec!["/Users/g/Astro/src/lib.rs".into()];
        s.test_nudge_count = 1;
        filter_test_pending_for_project(&mut s, "");
        assert_eq!(s.test_files_pending.len(), 1);
        assert_eq!(s.test_nudge_count, 1);
    }
}
