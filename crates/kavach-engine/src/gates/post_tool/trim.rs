//! Output-trimming check + the minimal state tracking used when trimming
//! short-circuits the normal handler flow (handlers write stdout; this does not).
use kavach_types::HookInput;

use crate::gates::{post_tool_bash, result_trim};

/// Check if tool output should be trimmed or sanitized to save context budget and prevent leaks.
pub(super) fn check_trim(input: &HookInput) -> Option<String> {
    let response_text = input
        .tool_response
        .as_ref()
        .and_then(|r| r.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if response_text.is_empty() {
        return None;
    }

    let file_path = input.get_string("file_path");
    let file_hint = if file_path.is_empty() { None } else { Some(file_path) };
    let sanitized = crate::gates::secret_redactor::sanitize_output(response_text, file_hint);
    let secrets_redacted = sanitized != response_text;

    // Check CCR caching for oversized payloads (>6KB).
    let (ccr_processed, cached_hash) = crate::ccr::process_ccr(&sanitized);
    if cached_hash.is_some() {
        return Some(ccr_processed);
    }

    let trimmed = match input.tool_name.as_str() {
        "Bash" => result_trim::trim_bash_output(&sanitized),
        "Glob" => result_trim::trim_glob_output(&sanitized),
        _ => None,
    };

    trimmed.or_else(|| secrets_redacted.then_some(sanitized))
}

/// Minimal state tracking when trimming short-circuits normal handler flow.
/// Does NOT write to stdout — only updates session state.
pub(super) fn track_state_only(input: &HookInput, session: &mut kavach_session::SessionState) {
    match input.tool_name.as_str() {
        "WebSearch" | "WebFetch" => {
            session.mark_research_done();
        }
        "Skill" => {
            let skill_name = input.get_string("skill");
            if !skill_name.is_empty() {
                session.record_skill_invoked(skill_name);
            }
        }
        "Bash" => {
            let command = input.get_string("command");
            if post_tool_bash::is_test_command_pub(command) {
                session.clear_test_pending();
            }
        }
        _ => {}
    }
}
