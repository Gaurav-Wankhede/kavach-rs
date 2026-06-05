//! `[RCA]` block detection in assistant prose + durable `rca.<slug>` decision-row
//! recognition. Pure functions for unit testability.

/// True when the last assistant message contains an `[RCA]` header.
/// Case-insensitive; tolerates extra whitespace and dashes.
pub(in crate::gates) fn has_rca_block(msg: &str) -> bool {
    if msg.is_empty() {
        return false;
    }
    let lower = msg.to_lowercase();
    lower.contains("[rca]")
        || lower.contains("[rca ")
        || lower.contains("[rca:")
        || lower.contains("[rca\n")
}

/// True iff `line` is a transcript line where the agent wrote a `kavach` decision
/// row keyed `rca.<slug>` — durable RCA satisfaction per §6 evidence chain.
/// All three tokens must co-occur on the same line; reduces FP from prose
/// merely mentioning the command. Pure fn for unit testability.
///
/// # Sources
/// - `~/.claude/CLAUDE.md` §6.2 REALITY-NOT-CLAIM
/// - reviewer P1 finding 2026-05-10 (Issue 5 of post-incident review)
pub(super) fn line_persists_rca_decision(line: &str) -> bool {
    line.contains("kavach db write")
        && line.contains("--category decision")
        && line.contains("rca.")
}
