//! `SessionStart` GUI bring-up: ensure the kavach web UI (port `7777`) and its
//! `SurrealDB` backend are running at the START of the session, autonomously —
//! not left to a manual `kavach servers up`. Detached + idempotent: `servers up`
//! no-ops when the port already listens, so re-running is harmless.
//! SOURCE: decision.session-start-autostarts-gui.

/// Spawn `kavach servers up` detached so the GUI on `:7777` (and `SurrealDB`) come
/// up when the session begins. Fire-and-forget: a spawn failure must never block
/// the `SessionStart` hook (which carries boot duties). Skipped under test.
pub(super) fn ensure_gui_up() {
    if cfg!(test) {
        return;
    }
    // `servers up` is itself idempotent (checks lsof, no-ops if :7777 listens), so
    // this is safe to fire on every session start. Detached: stdio nulled so the
    // child outlives the hook process.
    let spawned = std::process::Command::new("kavach")
        .args(["servers", "up"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    drop(spawned);
}
