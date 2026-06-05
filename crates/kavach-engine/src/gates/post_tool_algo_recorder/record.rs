//! Top-level `record` orchestrator: gate on file type, parse, verify, persist.
//! The DB/event/graph/enrich pipeline lives in the `persist` submodule.
mod persist;

use super::parse::extract_algo_comment;
use super::verify::verify_algo_comment;

/// Record an algorithm decision after a Write/Edit to a `.rs` file.
/// Verifies comment fields before upsert — rejects stale/invalid decisions.
/// Fire-and-forget on DB/event errors — recorder failure never blocks the gate.
/// `turn` is the harness turn counter; persisted on every event row so the audit
/// trail can correlate algorithm decisions with their producing turn.
pub(in crate::gates) fn record(file_path: &str, content: &str, project_slug: &str, turn: i64) {
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return;
    }
    let Some(algo) = extract_algo_comment(content) else {
        return;
    };
    if let Err(reason) = verify_algo_comment(&algo) {
        persist::reject(&algo, file_path, project_slug, turn, &reason);
        return;
    }
    persist::upsert(&algo, file_path, project_slug, turn);
    persist::write_edges(&algo, file_path);
    persist::trigger_enrich();
}
