//! Async, internet-first research kickoff.
//!
//! The intent gate runs under a ~3s `UserPromptSubmit` hook budget, so it can
//! not block on a live web search. Instead it fires [`kickoff`], which spawns a
//! detached thread that performs the (blocking) advisor web-search call and
//! writes the findings to a turn-scoped cache file. Downstream gates and the
//! next turn's injection read that cache via [`read_findings`] — the gate layer
//! never touches the network itself.
//!
//! Cache path: `<dir>/research/<session_id>.json` where `<dir>` is
//! `KAVACH_HOME` if set, else `~/.kavach`. One file per session, overwritten
//! each new researched prompt (the gate resets it at prompt start).

use std::fs;
use std::path::PathBuf;
use std::thread;

use serde::{Deserialize, Serialize};

use crate::client::ask;

/// Max advisor web searches per kickoff. Bounded to keep latency + cost sane.
const MAX_WEB_USES: u8 = 3;

/// Cached research findings for one session/turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct Findings {
    /// The topic that was researched.
    pub topic: String,
    /// Status: "pending" while the thread runs, "done" once written, "error" on failure.
    pub status: String,
    /// The synthesized research text (empty until done).
    pub summary: String,
}

/// Resolve the per-session research cache file path.
#[must_use]
pub fn cache_path(session_id: &str) -> PathBuf {
    let base = std::env::var_os("KAVACH_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".kavach")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("research").join(format!("{session_id}.json"))
}

/// Persist findings to `path`. Returns the IO error if the cache could not be
/// written — callers decide whether that is benign (it is, for the gate: a
/// missing cache reads as "no evidence" and the gate blocks, the safe default).
fn write_at(path: &std::path::Path, findings: &Findings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string(findings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

fn read_at(path: &std::path::Path) -> Option<Findings> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Write findings for a session. An IO failure is INTENTIONALLY discarded: the
/// research cache is advisory, and the pre-write gate fails safe on its absence
/// (no cache ⇒ "no evidence" ⇒ the edit is blocked, never silently allowed).
fn write_state(session_id: &str, findings: &Findings) {
    drop(write_at(&cache_path(session_id), findings));
}

/// Fire an internet-first research request in the background.
///
/// Returns immediately. Writes a `pending` marker now, then `done`/`error` when
/// the detached thread finishes. Never blocks the caller, never panics the host.
pub fn kickoff(session_id: &str, topic: &str) {
    let pending = Findings {
        topic: topic.to_owned(),
        status: "pending".to_owned(),
        summary: String::new(),
    };
    write_state(session_id, &pending);

    let sid = session_id.to_owned();
    let topic_owned = topic.to_owned();
    let worker = move || {
        let prompt = format!(
            "Research this engineering task using current authoritative web sources. \
             Return a concise findings brief with at least one source URL.\n\nTASK: {topic_owned}"
        );
        let result = match ask(&prompt, MAX_WEB_USES) {
            Ok(text) => Findings {
                topic: topic_owned,
                status: "done".to_owned(),
                summary: text,
            },
            Err(e) => Findings {
                topic: topic_owned,
                status: "error".to_owned(),
                summary: format!("research failed: {e}"),
            },
        };
        write_state(&sid, &result);
    };
    // INTENTIONAL JoinHandle discard: fire-and-forget. Spawn failure leaves the
    // cache `pending` ⇒ gate reads "no evidence" ⇒ blocks (fail-safe).
    drop(thread::Builder::new().name("kavach-research".to_owned()).spawn(worker));
}

/// Read current findings for a session, if the cache file exists and parses.
#[must_use]
pub fn read_findings(session_id: &str) -> Option<Findings> {
    read_at(&cache_path(session_id))
}

/// Clear the research cache for a session (called at new-prompt reset).
/// A missing file is the desired post-state, so the remove error is discarded.
pub fn clear(session_id: &str) {
    drop(fs::remove_file(cache_path(session_id)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_ends_with_session_json() {
        let p = cache_path("sess_abc");
        assert!(p.ends_with("research/sess_abc.json"));
    }

    #[test]
    fn read_at_none_when_absent() {
        assert!(read_at(std::path::Path::new("/tmp/kavach-no-such-xyz.json")).is_none());
    }

    #[test]
    fn write_then_read_roundtrips() {
        let dir = std::env::temp_dir().join("kavach-advisor-rt");
        let path = dir.join("sess_rt.json");
        let f = Findings {
            topic: "axum 0.8 middleware".to_owned(),
            status: "done".to_owned(),
            summary: "use tower::Layer; see https://docs.rs/axum".to_owned(),
        };
        write_at(&path, &f).expect("write should succeed");
        let got = read_at(&path).expect("should read back");
        assert_eq!(got.status, "done");
        assert!(got.summary.contains("https://"));
        let _ = fs::remove_file(&path);
    }
}
