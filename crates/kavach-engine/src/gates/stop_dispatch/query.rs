//! Next-dispatchable selectors: task / hunt / backlog, with fail-closed sentinel.
use super::card::SOURCE_DOWN_KEY;
use super::daemon::{rpc_census_only, rpc_get_directive, rpc_next, rpc_next_only, rpc_open_census};

/// The DB key (category `decision`) holding the project's DYNAMIC dispatch
/// directive — the operator-editable instruction text the stop-gate emits in place
/// of hardcoded prose. Per-project; absent → the gate uses a minimal fallback.
const DISPATCH_DIRECTIVE_KEY: &str = "gate.dispatch_directive";

/// Project's dynamic dispatch-directive text, or `None` to use the fallback.
/// The binary carries NO procedure prose — it fetches the instruction from the
/// DB so each project (and the operator) controls gate behavior without a rebuild.
pub(crate) fn next_task_directive(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    rpc_get_directive(project_slug, DISPATCH_DIRECTIVE_KEY)
}

/// DB-C — DYNAMIC GATE INJECTION (the core ask, operator directive 2026-06-18).
/// Resolve a gate's runtime advisory text from its `gate.injection.<gate_name>`
/// DB row (category `decision`). Generalizes the proven `gate.dispatch_directive`
/// pattern to EVERY gate: the binary ships NO advisory prose for `gate_name`; it
/// fetches the text from the DB at runtime, so the operator hot-edits any gate's
/// guidance with one `kavach db write` — no rebuild. FAIL-OPEN: an empty slug, a
/// missing row, or a down daemon all return `None`, and the caller emits nothing
/// (an absent injection NEVER blocks the gate — awareness is additive, not gating).
#[must_use]
pub(crate) fn gate_injection(project_slug: &str, gate_name: &str) -> Option<String> {
    if project_slug.is_empty() || gate_name.is_empty() {
        return None;
    }
    let key = format!("gate.injection.{gate_name}");
    rpc_get_directive(project_slug, &key).filter(|s| !s.trim().is_empty())
}

/// The DB key (category `decision`) naming the project's reward-rubric STACK
/// preset (`rust-cargo` / `ts-bun` / `python-uv`). Per-project; absent → the
/// RLAIF scorer uses the Rust default. DATA, no rebuild (mirrors the directive).
const REWARD_RUBRIC_KEY: &str = "gate.reward_rubric";

/// Resolve the project's RLAIF reward rubric from its `gate.reward_rubric` DB row
/// (a stack-preset name). Empty slug / absent row / unknown name all fall back to
/// the Rust-cargo default via [`kavach_patterns::reward::presets::by_name`], so a
/// non-Rust project (TS, Python) scores its own stack's verify commands instead
/// of zero. Operator directive 2026-06-17: "each project has different tech stacks".
#[must_use]
pub(crate) fn reward_rubric_for(project_slug: &str) -> kavach_patterns::reward::rubric::RewardRubric {
    let name = if project_slug.is_empty() {
        String::new()
    } else {
        rpc_get_directive(project_slug, REWARD_RUBRIC_KEY).unwrap_or_default()
    };
    kavach_patterns::reward::presets::by_name(name.trim())
}

/// DB key for the multidimensional-oracle config override (a JSON blob of weight /
/// margin / penalty / failure-vocab overrides). Cross-project harness doctrine.
const REWARD_ORACLE_KEY: &str = "gate.reward_oracle";

/// Resolve the project's ground-truth oracle config from its `gate.reward_oracle`
/// DB row (a JSON object whose present fields override the compiled defaults). This
/// is what makes the oracle's weights / penalty / failure-vocab DATA, not source
/// literals — research-refreshable per project. Empty slug / absent row / malformed
/// JSON all fall back to [`OracleConfig::default`] (the compiled fail-safe), so the
/// oracle is never worse than its hardcoded floor when the DB is unreachable.
#[must_use]
pub(crate) fn oracle_config_for(project_slug: &str) -> kavach_patterns::reward::oracle::OracleConfig {
    use kavach_patterns::reward::oracle::OracleConfig;
    if project_slug.is_empty() {
        return OracleConfig::default();
    }
    let Some(json) = rpc_get_directive(project_slug, REWARD_ORACLE_KEY) else {
        return OracleConfig::default();
    };
    // `#[serde(default)]` on OracleConfig fills every field the row omits from the
    // compiled default, so a partial override is honored and a malformed blob
    // degrades to the full default — never a panic, never a zeroed config.
    serde_json::from_str(json.trim()).unwrap_or_default()
}

/// DB key for the done-gaming vocabulary override (a JSON object with optional
/// `gaming_phrases` / `handback_phrases` string arrays). Cross-project harness
/// doctrine — the Stop gate's completion-narration + operator-handback markers as
/// DATA, research-refreshable per project, NOT source literals.
const DONE_GAMING_VOCAB_KEY: &str = "gate.done_gaming_vocab";

/// Resolve the project's done-gaming vocabulary from its `gate.done_gaming_vocab`
/// DB row. This is what connects the Stop gate's done-detection to the kavach DB +
/// Brain-OS instead of compiled `const` arrays: the operator (or a research refresh)
/// hot-edits the gaming/handback phrase lists with one `kavach db write`, no rebuild.
/// Empty slug / absent row / malformed JSON all fall back to
/// [`DoneGamingVocab::default`] (the compiled phrase floor), so the gate is never
/// weaker than its hardcoded baseline when the DB is unreachable. Mirrors
/// [`oracle_config_for`].
#[must_use]
pub(crate) fn done_gaming_vocab_for(
    project_slug: &str,
) -> kavach_patterns::stop_vocab::DoneGamingVocab {
    use kavach_patterns::stop_vocab::DoneGamingVocab;
    if project_slug.is_empty() {
        return DoneGamingVocab::default();
    }
    let Some(json) = rpc_get_directive(project_slug, DONE_GAMING_VOCAB_KEY) else {
        return DoneGamingVocab::default();
    };
    // `#[serde(default)]` fills each list the row omits from the compiled default,
    // so a partial override (only `gaming_phrases`) keeps the default handback list,
    // and a malformed blob degrades to the full default — never a panic, never empty.
    serde_json::from_str(json.trim()).unwrap_or_default()
}

fn source_down_sentinel() -> (String, String) {
    (
        SOURCE_DOWN_KEY.to_owned(),
        "kanban source UNREACHABLE (RPC + direct DB both failed) — cannot \
         verify empty; assume work pending, do NOT stop"
            .to_owned(),
    )
}

fn label_from(val: &serde_json::Value) -> Option<(String, String)> {
    let key = val.get("key").and_then(|s| s.as_str())?.to_owned();
    let title = val.get("title").and_then(|s| s.as_str())?.to_owned();
    let status = val
        .get("status")
        .and_then(|s| s.as_str())
        .map_or_else(String::new, str::to_owned);
    Some((key, format!("{title} [{status}]")))
}

/// Next dispatchable roadmap task (key, `title [status]`), or None when empty.
/// `SOURCE_DOWN` sentinel on RPC outage (caller fails closed).
pub(crate) fn get_next_task_info(project_slug: &str) -> Option<(String, String)> {
    select(project_slug, "roadmap.next_open_task")
}

/// Next open bug-hunt card (roadmap row, key prefix 'hunt.', open status).
/// The harness cannot stop while a proven, unfixed defect remains.
pub(crate) fn get_next_hunt_info(project_slug: &str) -> Option<(String, String)> {
    select(project_slug, "roadmap.next_open_hunt")
}

/// Continuous-pipeline refill: priority-ordered runnable backlog head, so the
/// loop keeps draining the roadmap rather than halting on an empty open-set.
pub(crate) fn get_next_backlog_info(project_slug: &str) -> Option<(String, String)> {
    select(project_slug, "roadmap.promote_next_backlog")
}

/// Open-set census distinguishing a BLOCKED remainder from a truly empty board.
/// `(runnable, blocked, cyclic)` counts of dispatch-status cards / those held back
/// by unmet deps / those in a dependency cycle. `None` on empty
/// slug or RPC outage — the caller fails closed (treats an unobservable board as
/// "do not clean-stop").
pub(crate) fn open_set_census(project_slug: &str) -> Option<(u64, u64, u64)> {
    if project_slug.is_empty() {
        return None;
    }
    match rpc_open_census(project_slug) {
        Ok(Some((r, b, c))) => Some((r, b, c)),
        Ok(None) | Err(()) => None,
    }
}

/// RPC-ONLY census for HOT entry hooks (`SessionStart` / `UserPromptSubmit`).
/// Single bounded RPC call — NO daemon self-heal, NO direct-DB cold-open. `None`
/// on empty slug or any outage, so the hook fails SOFT and FAST to the legacy nag
/// instead of blocking the session on a `RocksDB` open. Distinct from
/// [`open_set_census`], which is the Stop gate's already-warm drained-branch read.
pub(crate) fn census_rpc_only(project_slug: &str) -> Option<(u64, u64, u64)> {
    if project_slug.is_empty() {
        return None;
    }
    rpc_census_only(project_slug)
}

/// RPC-ONLY next-task name for hot entry hooks: bounded single call, no fallback.
/// `None` on empty slug or outage → the caller omits the "next card" line.
pub(crate) fn next_task_rpc_only(project_slug: &str) -> Option<(String, String)> {
    if project_slug.is_empty() {
        return None;
    }
    rpc_next_only(project_slug).as_ref().and_then(label_from)
}

/// Shared selector body: empty slug → None; RPC down → fail-closed sentinel.
fn select(project_slug: &str, method: &str) -> Option<(String, String)> {
    if project_slug.is_empty() {
        return None;
    }
    match rpc_next(method, project_slug) {
        Ok(Some(v)) => label_from(&v),
        Ok(None) => None,
        Err(()) => Some(source_down_sentinel()), // RPC down -> fail closed
    }
}
