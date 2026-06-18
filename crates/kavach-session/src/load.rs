use std::fs;
use std::io;

use crate::parse::parse_field;
use crate::paths::{ensure_parent_dir, state_path, state_path_for, today};
use crate::state::SessionState;

/// Load session state from the state file.
///
/// # Errors
///
/// Returns `Err` if the state file exists but cannot be read due to I/O errors
/// (e.g., filesystem permissions, file corruption).
pub fn load_session_state() -> Result<Option<SessionState>, io::Error> {
    let path = state_path();
    if !path.exists() {
        return Ok(None);
    }

    // Shared lock prevents reading while a save() holds exclusive lock.
    // Multiple readers can proceed concurrently (shared is non-exclusive).
    let lock_path = path.with_extension("lock");
    ensure_parent_dir(&lock_path)?;
    let lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&lock_path)?;
    lock_file.lock_shared()?;

    let content = fs::read_to_string(&path)?;

    lock_file.unlock()?;
    let state = parse_ini_str(&content);

    if state.today != today() {
        return Ok(None);
    }

    Ok(Some(state))
}

/// Parse INI session-state text into a `SessionState`.
///
/// Shared by the file loader and the DB-blob loader.
/// `session_runtime.state_blob` stores exactly the `to_ini_full()` output,
/// so the same parser round-trips both.
#[must_use]
pub fn parse_ini_str(content: &str) -> SessionState {
    let mut state = SessionState::default();
    let mut in_files = false;
    let mut in_case_facts = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            in_files = false;
            in_case_facts = trimmed == "[CASE_FACTS]";
            continue;
        }
        if in_files && trimmed.starts_with("- ") {
            state
                .files_modified
                .push(trimmed.strip_prefix("- ").unwrap_or(trimmed).to_owned());
            continue;
        }
        if in_case_facts && trimmed.starts_with("- ") {
            state
                .case_facts
                .push(trimmed.strip_prefix("- ").unwrap_or(trimmed).to_owned());
            continue;
        }
        if let Some(idx) = trimmed.find(':') {
            let (key_part, rest) = trimmed.split_at(idx);
            let key = key_part.trim();
            // rest starts with ':', safe to slice off first char
            let value = rest.get(1..).map_or("", str::trim);
            parse_field(&mut state, key, value, &mut in_files);
        }
    }
    state
}

/// Session-aware load: durable runtime state keyed by `session_id`.
///
/// Resolution order:
/// 1. RPC `session.get` for THIS `session_id` — durable cross-`/clear` resume.
/// 2. Fall back to the INI file, but ONLY if its stored `session_id` matches
///    the requested one. A mismatch means the file belongs to a PRIOR
///    conversation (the `/clear` rehydration drift) — return fresh state, never
///    the stale row.
/// 3. No match anywhere ⇒ `None` (caller starts fresh).
///
/// Fail-open: an RPC error is not fatal — fall through to the INI check so a
/// dead daemon degrades to today's file-only behavior rather than blocking.
#[must_use]
pub fn load_session_state_for(session_id: &str) -> Option<SessionState> {
    if session_id.is_empty() {
        return load_session_state().ok().flatten();
    }
    // `session.get` returns `null` (no row) or `{session_id,workdir,state_blob}`.
    // Parse the blob out of a serde_json::Value — no derive, no `serde` dep.
    let params = serde_json::json!({ "session_id": session_id });
    match kavach_rpc::client::call::<_, serde_json::Value>("session.get", Some(params)) {
        Ok(value) => {
            if let Some(blob) = value.get("state_blob").and_then(|b| b.as_str()) {
                let state = parse_ini_str(blob);
                if state.today == today() {
                    return Some(state);
                }
            }
        }
        Err(e) => {
            // RPC error — daemon is down or unreachable. Log it but continue to INI fallback.
            tracing::warn!(error = ?e, "kavach-session: session.get failed, falling back to INI");
        }
    }
    // DB miss or daemon down — fall back to the conversation-scoped INI file.
    match load_session_state_at(state_path_for(session_id).as_path()) {
        Ok(Some(state)) if state.session_id == session_id => Some(state),
        // Mismatch or missing: refuse a stale or foreign row.
        _ => None,
    }
}

/// Load session state from a specific INI path (shared lock).
fn load_session_state_at(path: &std::path::Path) -> Result<Option<SessionState>, io::Error> {
    if !path.exists() {
        return Ok(None);
    }
    let lock_path = path.with_extension("lock");
    ensure_parent_dir(&lock_path)?;
    let lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&lock_path)?;
    lock_file.lock_shared()?;
    let content = fs::read_to_string(path)?;
    lock_file.unlock()?;
    let state = parse_ini_str(&content);
    if state.today != today() {
        return Ok(None);
    }
    Ok(Some(state))
}

pub(crate) fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_csv() {
        assert_eq!(split_csv("a, b, c"), vec!["a", "b", "c"]);
        assert_eq!(split_csv(""), Vec::<String>::new());
        assert_eq!(split_csv("solo"), vec!["solo"]);
    }

    #[test]
    fn parse_ini_str_round_trips_to_ini_full() {
        // session_runtime.state_blob stores to_ini_full() output; parse_ini_str
        // must reconstruct an equivalent state from it.
        let mut original = SessionState::new("/tmp/work");
        original.session_id = "sess_round_trip".into();
        original.research_done = true;
        original.files_modified = vec!["a.rs".into(), "b.rs".into()];
        let ini = original.to_ini_full();
        let parsed = parse_ini_str(&ini);
        assert_eq!(parsed.session_id, "sess_round_trip");
        assert!(parsed.research_done);
        assert_eq!(parsed.files_modified, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn parse_ini_str_empty_yields_default() {
        let parsed = parse_ini_str("");
        assert!(parsed.session_id.is_empty());
        assert!(parsed.files_modified.is_empty());
    }

    #[test]
    fn load_for_empty_session_id_falls_back_to_file_path() {
        // An empty session_id has no DB row to key on — must defer to the
        // file-only loader rather than panic or query with an empty key.
        // (Returns None here only because no state file exists in the test
        // env; the contract is "no crash, file-path semantics".)
        drop(load_session_state_for(""));
    }
}
