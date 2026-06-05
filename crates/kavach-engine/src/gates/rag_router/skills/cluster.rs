//! Imperative NLU: intent type → mandatory cluster skills, injected regardless
//! of RAG score. Loads dynamically from the kavach-db `intent_cluster` pattern.
use std::process::Command;

/// Inject cluster skills for `intent` into `skills` (no duplicates).
pub(super) fn inject_intent_cluster(skills: &mut Vec<String>, intent: &str) {
    for skill in intent_cluster_skills(intent) {
        if !skills.iter().any(|s| s == &skill) {
            skills.push(skill);
        }
    }
}

/// Intent type → cluster skills. INVARIANT: must not include skills with
/// `user-invocable: false` — those cannot be satisfied via the Skill tool.
/// Falls back to `evidence-chain` (always safe) if the db is unavailable.
fn intent_cluster_skills(intent: &str) -> Vec<String> {
    rpc_intent_cluster(intent).unwrap_or_else(|| vec!["evidence-chain".to_owned()])
}

/// Fetch the intent cluster from kavach-db `memory_entries` via `kavach db get`.
/// Row at `category=pattern, key=intent_cluster.<intent>`; content JSON carries
/// `fix_action: [...]`. `None` on missing row, CLI failure, or malformed JSON
/// (fail-closed: silent degradation to fallback in the caller).
fn rpc_intent_cluster(intent: &str) -> Option<Vec<String>> {
    let project = std::env::var("KAVACH_PROJECT").ok()?;
    let key = format!("intent_cluster.{intent}");
    let output = Command::new("kavach")
        .args([
            "db",
            "get",
            "--project",
            &project,
            "--category",
            "pattern",
            "--key",
            &key,
            "--full",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: header lines, then "---", then JSON content.
    let json_str = stdout.split("---").nth(1)?.trim();
    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let arr = val.get("fix_action")?.as_array()?;
    let skills: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
        .collect();
    (!skills.is_empty()).then_some(skills)
}
