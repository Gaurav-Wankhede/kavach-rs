//! Per-conversation session-id resolution for the session loader.
//!
//! Claude Code sets `KAVACH_SESSION_ID` on every hook; Cursor does NOT — it
//! carries the id as `conversation_id`, which the native hook edge lowers into
//! `HookInput.session_id` and arms here via [`set_session_context`]. Resolving
//! through this cell (not env mutation, which is `unsafe` in edition 2024 and
//! racy) lets EVERY `get_or_create_session()` call site key the durable row +
//! stop-reblock counter per conversation without threading the id through ~20
//! signatures.

std::thread_local! {
    /// The session id for THIS thread's gate invocation, set once by the native
    /// hook edge from the lowered `HookInput.session_id`.
    static SESSION_ID_CTX: std::cell::RefCell<String> =
        const { std::cell::RefCell::new(String::new()) };
}

/// Arm the session-id context for all subsequent session loads on THIS thread.
///
/// Called by the native hook edge after lowering the payload, so a Cursor
/// `conversation_id` (which sets no env var) still keys the durable session
/// row + the stop-reblock counter per conversation. No-op for the empty id.
pub fn set_session_context(session_id: &str) {
    SESSION_ID_CTX.with(|c| session_id.clone_into(&mut c.borrow_mut()));
}

/// Resolve the session id: prefer the `KAVACH_SESSION_ID` env (Claude Code sets
/// it on every hook), else the thread-local context the edge armed from the
/// lowered input (Cursor's `conversation_id`). Empty if neither is set —
/// callers guard on emptiness.
pub(super) fn env_session_id() -> String {
    let env = std::env::var("KAVACH_SESSION_ID").unwrap_or_default();
    if !env.is_empty() {
        return env;
    }
    SESSION_ID_CTX.with(|c| c.borrow().clone())
}

#[cfg(test)]
#[path = "session_id_test.rs"]
mod tests;
