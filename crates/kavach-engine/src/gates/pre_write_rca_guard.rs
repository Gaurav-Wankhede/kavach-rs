// RCA enforcement gate: blocks debug/refactor/implement without [RCA] block.
// See decision.engine.rca_protocol_enforcement.
mod bypass;
mod detect;
mod rules;
#[cfg(test)]
#[path = "pre_write_rca_guard_test.rs"]
mod tests;
mod transcript;
use bypass::active_bulk_sweep;
pub(in crate::gates) use detect::has_rca_block;
use rules::{REQUIRES_RCA, is_rca_exempt_path, risk_requires_rca};
pub(in crate::gates) use transcript::scan_transcript_for_rca;
/// Returns `Some(reason)` when the gate should block, `None` when allow.
/// SOURCE: pattern matches existing `pre_write_*_guard.rs` return convention.
/// `session_rca_present` carries persistent multi-turn signal from session state.
/// Gate passes when EITHER session flag OR current message has `[RCA]` OR active bulk sweep OR exempt path.
pub(crate) fn check(
    tool_name: &str,
    intent_type: &str,
    intent_risk: &str,
    last_assistant_message: &str,
    session_rca_present: bool,
    file_path: &str,
) -> Option<String> {
    if !matches!(tool_name, "Edit" | "Write" | "NotebookEdit") {
        return None;
    }
    if is_rca_exempt_path(file_path) {
        return None;
    }
    if !REQUIRES_RCA.contains(&intent_type) {
        return None;
    }
    if !risk_requires_rca(intent_risk) {
        return None;
    }
    if bypass_allows(tool_name, intent_type, intent_risk, file_path) {
        return None;
    }
    if session_rca_present || has_rca_block(last_assistant_message) {
        return None;
    }
    Some(format!(
        "[RCA_FIRST] {intent_type}/{intent_risk} → emit [RCA] in CHAT (never a code comment): \
         symptom@file:line → why-chain→root_cause · class+blast · fix · cite:URL → retry \
         this edit. Fix cause ≠ symptom; non-obvious WHY gets ≤1 line in code. \
         SOURCE: CLAUDE.md §RCA."
    ))
}
/// True when an active bulk sweep authorizes this edit.
/// Emits a structured stderr audit line (the hook engine's log channel).
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
fn bypass_allows(tool_name: &str, _intent_type: &str, _intent_risk: &str, file_path: &str) -> bool {
    // Bulk-mode: a manifest signed at sweep boundary (kavach bulk start, with
    // explicit user approval) carries the shared RCA + scope_glob + fix_strategy.
    // Per-Edit we skip the RCA demand; post-write emits bulk_apply tagged with
    // sweep_id. Daemon-side conformance check verifies file+strategy match.
    if let Some(sweep_id) = active_bulk_sweep() {
        eprintln!(
            "[KAVACH_BULK] sweep={sweep_id} authorizes edit (tool={tool_name}, file={file_path})"
        );
        return true;
    }
    false
}
