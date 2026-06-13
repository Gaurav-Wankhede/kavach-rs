//! Session load/create hub.
//!
//! Resolves a session id (env or thread-local edge context — see
//! [`session_id`]) and loads the durable per-conversation row, so two
//! conversations in one repo never collide on the workdir-keyed file cache.

mod filter;
mod session_id;

use crate::load::{load_session_state, load_session_state_for};
use crate::paths::detect_project;
use crate::state::SessionState;

pub(crate) use filter::filter_test_pending_for_project;
use session_id::env_session_id;
pub use session_id::set_session_context;

/// Load existing session or create a new one.
///
/// Resolves the session id from `KAVACH_SESSION_ID` (Claude Code) or the
/// thread-local context the native edge armed from the lowered payload
/// (Cursor's `conversation_id`, which sets no env). With an id present this
/// routes to the durable per-conversation row via [`get_or_create_session_for`],
/// so two conversations in ONE repo never share the workdir-keyed file cache —
/// the collision that made two Cursor conversations advance one loop counter.
/// Empty id (no edge, no env) falls back to the file-only load, today's
/// behavior.
#[must_use]
pub fn get_or_create_session() -> SessionState {
    let session_id = env_session_id();
    if session_id.is_empty() {
        materialize(load_session_state().ok().flatten())
    } else {
        get_or_create_session_for(&session_id)
    }
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
