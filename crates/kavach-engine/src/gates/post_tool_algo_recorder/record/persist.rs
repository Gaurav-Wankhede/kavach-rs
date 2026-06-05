//! Persistence pipeline for a verified (or rejected) algorithm decision: row
//! upsert, event append, graph edges, RAG enrich. All fire-and-forget.
use std::io::Write;
use std::process::Command;

use super::super::parse::AlgoComment;

fn rpc_entity_upsert(entity_type: &str, name: &str) -> Option<String> {
    let params = serde_json::json!({"entity_type": entity_type, "name": name});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.entity_upsert", Some(params));
    result
        .ok()
        .and_then(|v| v.get("id").and_then(|s| s.as_str()).map(ToOwned::to_owned))
}

fn rpc_add_relationship(from: &str, to: &str, rel_type: &str, weight: f64) {
    let params = serde_json::json!({
        "from": from, "to": to, "rel_type": rel_type, "weight": weight,
    });
    kavach_rpc::client::call::<_, serde_json::Value>("graph.add_relationship", Some(params)).ok();
}

/// Append a rejection event + log to stderr (the hook log channel).
pub(super) fn reject(
    algo: &AlgoComment,
    file_path: &str,
    project_slug: &str,
    turn: i64,
    reason: &str,
) {
    let payload = format!(
        r#"{{"chosen":"{}","file":"{file_path}","reason":"{reason}","turn":{turn}}}"#,
        algo.chosen
    );
    let event_params = serde_json::json!({
        "event_type": "algorithm_decision_rejected",
        "source": "post_write_algo_recorder",
        "project": project_slug,
        "payload": payload,
    });
    kavach_rpc::client::call::<_, serde_json::Value>("event.append", Some(event_params)).ok();
    drop(writeln!(
        std::io::stderr(),
        "[ALGO_VERIFY_FAIL turn={turn}] {reason}"
    ));
}

/// Upsert the algo decision + append the success event.
pub(super) fn upsert(algo: &AlgoComment, file_path: &str, project_slug: &str, turn: i64) {
    let upsert_params = serde_json::json!({
        "project": project_slug,
        "problem_class": algo.problem_class,
        "chosen": algo.chosen,
        "time_complexity": algo.time_complexity,
        "space_complexity": algo.space_complexity,
        "file_path": file_path,
        "search_year": i32::try_from(algo.search_year).unwrap_or(i32::MAX),
        "search_month": i32::try_from(algo.search_month).unwrap_or(i32::MAX),
        "turn": turn,
    });
    kavach_rpc::client::call::<_, serde_json::Value>("algo.upsert", Some(upsert_params)).ok();

    let payload = format!(
        r#"{{"chosen":"{}","problem_class":"{}","file":"{file_path}","turn":{turn}}}"#,
        algo.chosen, algo.problem_class
    );
    let event_params = serde_json::json!({
        "event_type": "algorithm_decision",
        "source": "post_write_algo_recorder",
        "project": project_slug,
        "payload": payload,
    });
    kavach_rpc::client::call::<_, serde_json::Value>("event.append", Some(event_params)).ok();
}

/// Write graph edges: file→algorithm (`uses_algorithm`), algorithm→class (`solves`).
pub(super) fn write_edges(algo: &AlgoComment, file_path: &str) {
    if let (Some(fid), Some(aid), Some(cid)) = (
        rpc_entity_upsert("file", file_path),
        rpc_entity_upsert("algorithm", &algo.chosen),
        rpc_entity_upsert("problem_class", &algo.problem_class),
    ) {
        rpc_add_relationship(&fid, &aid, "uses_algorithm", 1.0);
        rpc_add_relationship(&aid, &cid, "solves", 1.0);
    }
}

/// Trigger RAG enrich on the arch skill — non-blocking subprocess.
pub(super) fn trigger_enrich() {
    let algo_skill_dir = kavach_config::paths::skills_dir().join("arch");
    if algo_skill_dir.exists() {
        Command::new("kavach")
            .args([
                "rag",
                "enrich",
                "--source",
                &algo_skill_dir.to_string_lossy(),
                "--label",
                "skills",
            ])
            .spawn()
            .ok();
    }
}
