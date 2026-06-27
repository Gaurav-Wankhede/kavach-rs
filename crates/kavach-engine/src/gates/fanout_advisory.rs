//! Fan-out nudge: when a FRONTIER model does Read/Edit/Write/Bash itself, remind it
//! to delegate the labor to the cheap executor tier. Advisory only — never blocks.
//! SOURCE: decision.harness.fanout-to-cheap-tier.
use kavach_session::SessionState;
const LABOR_TOOLS: &[&str] = &["Read", "Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"];
/// Return the one-per-turn `[FANOUT_NUDGE]` when `session`'s model is a frontier tier
/// and `tool_name` is a direct labor tool. `None` on the cheap tier (it IS the doer),
/// on a non-labor tool, or after the nudge already fired this turn. Sets the sent-flag
/// and persists when it fires so repeats stay silent.
pub(crate) fn nudge(session: &mut SessionState, tool_name: &str) -> Option<String> {
    let cheap = kavach_config::model::cheap_executor_tier();
    if session.fanout_nudge_sent
        || !LABOR_TOOLS.contains(&tool_name)
        || !kavach_config::model::is_frontier_tier(&session.model_id, &cheap)
    {
        return None;
    }
    session.fanout_nudge_sent = true;
    session.save().ok();
    Some(format!(
        "[FANOUT_NUDGE] You ran {tool_name} yourself on the frontier tier. You are the \
         ORCHESTRATOR — spawn a {cheap} agent to do the read/edit/run, then VERIFY what it \
         returns. Reserve your tokens for the decision, not the labor. (Carve-out: a single \
         trivial check where spawning costs more than it saves.) SOURCE: \
         anthropic.com/engineering/multi-agent-research-system."
    ))
}
#[cfg(test)]
#[path = "fanout_advisory_test.rs"]
mod tests;
