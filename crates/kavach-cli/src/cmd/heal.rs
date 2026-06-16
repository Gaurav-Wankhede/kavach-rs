//! `kavach heal` — self-healing pipeline orchestration (Kavach replaces N8N).
//! Kavach does ONLY non-AI ops: it captures failure context and writes a typed
//! self-heal roadmap card; the autonomous loop's subscription native agent then
//! claims + fixes it. Kavach NEVER calls a metered LLM.
//! SOURCE: decision.heal.self-healing-pipeline-architecture.

mod capture;
pub(crate) mod ingest;
pub(crate) mod merge_gate;
pub(crate) mod sweep;

/// `kavach heal capture` entry: gather context for `incident` and upsert its
/// self-heal card. `log_path` is the build/test log; `diff_base` is the git ref
/// to diff against (default `HEAD~1`). The card is written through the canonical
/// RPC-first write path (`db::upsert_roadmap_card`) so Kavach never opens a
/// second `RocksDB` handle and races the daemon (single-writer invariant).
pub(crate) fn run(
    project: &str,
    incident: &str,
    summary: &str,
    log_path: Option<&str>,
    diff_base: &str,
) -> i32 {
    // Read the log if given; a missing/unreadable log is non-fatal — the card is
    // still worth writing from the summary + diff (failure-tolerant by design).
    let log = log_path.map_or_else(String::new, |p| {
        std::fs::read_to_string(p).unwrap_or_else(|e| format!("(log unreadable: {e})"))
    });
    capture_incident(project, incident, summary, &log, diff_base)
}

/// Gather context for an incident (log already in memory) and upsert its
/// self-heal card via the RPC single-writer path. Shared by `capture` (one
/// incident from a file) and `sweep` (one per failing gate). Returns the CLI
/// exit code from the underlying write.
pub(crate) fn capture_incident(
    project: &str,
    incident: &str,
    summary: &str,
    log: &str,
    diff_base: &str,
) -> i32 {
    let inc = capture::gather(incident, summary, log, diff_base);
    let key = capture::card_key(&inc.id);
    let content = capture::card_content(&inc);
    crate::cmd::db::upsert_roadmap_card(project, &key, summary, &content)
}
