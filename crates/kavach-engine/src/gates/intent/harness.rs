//! L4 intent-gate harness classifier (Pattern-1: Classify-and-Act). Maps the
//! user prompt to one of the six dynamic-workflow harness patterns by keyword
//! routing, then persists the choice onto the project's next-open card via the
//! `db.set_harness` RPC so the L3 stop-gate dispatches that workflow.
//! SOURCE: decision.goal-harness-6-patterns · roadmap.unit.harness-loop-L4-classifier.
use serde_json::json;
/// Cheap doer tier (Haiku) — mirrors `goal::compile::model_tier::CHEAP_MODEL`.
pub(crate) const CHEAP_MODEL: &str = "claude-haiku-4-5";
/// Actionable per-pattern dispatch directive; parallel patterns name the Haiku tier + `Agent` spawn.
pub(crate) fn pattern_directive(pattern: &str) -> String {
    match pattern {
        "fan-out-synthesize" => format!(
            "Shard this across parallel `Agent` subagents on the cheap `{CHEAP_MODEL}` (Haiku) tier \
             — one shard each, then synthesize the shards on your frontier model."
        ),
        "generate-filter" => format!(
            "Generate candidates with parallel `Agent` subagents on `{CHEAP_MODEL}` (Haiku), \
             then filter to oracle-passing survivors on your frontier model."
        ),
        "pairwise-tournament" => format!(
            "Produce competitors with parallel `Agent` subagents on `{CHEAP_MODEL}` (Haiku), \
             then judge head-to-head on your frontier model until one champion remains."
        ),
        "worker-critic" => format!(
            "Produce the artifact, then spawn independent critic `Agent` subagents on \
             `{CHEAP_MODEL}` (Haiku) to adversarially grade it before you accept it."
        ),
        "classify-act" => {
            "Route each item to its handler by type — act inline, no fan-out needed.".to_owned()
        }
        _ => "Oracle-gated single loop: implement → verify the check → repeat until it passes."
            .to_owned(),
    }
}
/// The six harness patterns, in kebab-case (matches the `Harness` serde rename).
/// The single source of truth for the valid set; the test suite asserts every
/// classification is a member.
#[cfg(test)]
pub(crate) const PATTERNS: [&str; 6] = [
    "classify-act",
    "fan-out-synthesize",
    "worker-critic",
    "generate-filter",
    "pairwise-tournament",
    "loop-until-done",
];
/// Classify a prompt into a harness pattern (Pattern-1 Classify-and-Act). Keyword
/// routing over the lowered prompt; the default for open-ended build/fix work is
/// `loop-until-done` (the original goal-loop behavior, so nothing regresses).
#[must_use]
pub(crate) fn classify_harness(prompt: &str) -> &'static str {
    let p = prompt.to_lowercase();
    if has_any(
        &p,
        &["classify", "route", "triage", "categor", "dispatch by type"],
    ) {
        "classify-act"
    } else if has_any(
        &p,
        &[
            "audit",
            "sweep",
            "all files",
            "every ",
            "across the",
            "fan out",
            "parallel",
        ],
    ) {
        "fan-out-synthesize"
    } else if has_any(
        &p,
        &[
            "review",
            "critique",
            "verify",
            "adversari",
            "harden",
            "double-check",
        ],
    ) {
        "worker-critic"
    } else if has_any(
        &p,
        &[
            "brainstorm",
            "candidate",
            "options",
            "alternativ",
            "generate several",
        ],
    ) {
        "generate-filter"
    } else if has_any(
        &p,
        &[
            "compare",
            "best of",
            "tournament",
            "rank",
            "pick the winner",
            "versus",
        ],
    ) {
        "pairwise-tournament"
    } else {
        "loop-until-done"
    }
}
fn has_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}
/// Persist the classified harness onto the project's next-open card via
/// `db.set_harness`, returning the ready-to-append `[HARNESS]` context block.
/// Fail-soft: any RPC error is swallowed (classification is advisory; the loop
/// still dispatches the card without a harness).
pub(crate) fn persist_for_next_card(project: &str, prompt: &str) -> String {
    let pattern = classify_harness(prompt);
    if !project.is_empty()
        && let Ok(card) = kavach_rpc::client::call::<_, serde_json::Value>(
            "roadmap.next_open_task",
            Some(json!({ "project": project })),
        )
        && let Some(key) = card.get("key").and_then(serde_json::Value::as_str)
    {
        drop(kavach_rpc::client::call::<_, serde_json::Value>(
            "db.set_harness",
            Some(json!({ "project": project, "key": key, "harness": pattern })),
        ));
    }
    format!(
        "\n[HARNESS] classified -> {pattern} (persisted on next-open card; \
         the stop-gate will dispatch this workflow).\n{}",
        pattern_directive(pattern)
    )
}
#[cfg(test)]
#[path = "harness_test.rs"]
#[path = "harness_test.rs"]
mod tests;
