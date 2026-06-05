//! Tier-1 autonomous-pattern lookup/upsert via kavach-rpc, plus the Tier-2
//! `[SELF_EVOLVE]` advisory block for novel errors.

/// Build a `[SELF_EVOLVE]` advisory block for novel errors. Instructs Claude to
/// research the error, resolve it, then write the fix back to kavach-db so it
/// becomes an autonomous Tier 1 pattern.
pub(super) fn self_evolve_block(error: &str, tool_name: &str, failure_type: &str) -> String {
    format!(
        "[SELF_EVOLVE]\n\
         status: novel_error\n\
         tool: {tool_name}\n\
         failure_type: {failure_type}\n\
         error_fingerprint: {tokens}\n\
         action: Research this error class. Find the root cause, best-fit algorithm/DSA for \
         detection, and optimal fix strategy. Then write back:\n\
         \x20 kavach db write --project <slug> --category gate_pattern \\\n\
         \x20   --key \"fix-{tool_name}-<short-slug>\" \\\n\
         \x20   --title \"<imperative rewrite of block message>\" \\\n\
         \x20   --content \"fix_strategy: <strategy>\\ndsa_rationale: <why this algorithm>\"\n\
         reason: Unrecognised error — storing fix enables autonomous resolution next occurrence.\n",
        tokens = kavach_surreal::gate_pattern_tokenize(error),
    )
}

/// Query kavach-rpc for an autonomous `gate_pattern` matching this error.
/// Returns None if daemon not running or no match — caller falls through to Tier 2.
pub(super) fn find_autonomous_via_rpc(
    project_slug: &str,
    error_text: &str,
    tool_name: &str,
) -> Option<kavach_surreal::GatePattern> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({
        "project": project_slug,
        "error": error_text,
        "tool_name": tool_name,
    });
    kavach_rpc::client::call("gate_pattern.find_autonomous", Some(params)).ok()?
}

/// Upsert an autonomous-tier hit via kavach-rpc to bump `occurrence_count`.
/// Best-effort: errors are swallowed since the gate already injected context.
pub(super) fn upsert_via_rpc(
    project_slug: &str,
    error_text: &str,
    pat: &kavach_surreal::GatePattern,
    tool_name: &str,
) {
    let params = serde_json::json!({
        "project": project_slug,
        "error_tokens": error_text,
        "fix_strategy": pat.fix_strategy,
        "imperative_rewrite": pat.imperative_rewrite,
        "dsa_rationale": pat.dsa_rationale,
        "tool_name": tool_name,
        "gate_name": "post_tool_failure",
    });
    kavach_rpc::client::call::<_, serde_json::Value>("gate_pattern.upsert", Some(params)).ok();
}
