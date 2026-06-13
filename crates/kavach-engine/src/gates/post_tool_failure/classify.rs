//! Failure-type classification + generic per-type action guidance.

/// Classify failure type from error details for smart stop-gate decisions.
pub(super) fn classify_failure(error: &str) -> &'static str {
    let lower = error.to_lowercase();
    if lower.contains("rate limit")
        || lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("connection refused")
        || lower.contains("temporarily unavailable")
        || lower.contains("503")
        || lower.contains("429")
    {
        return "transient";
    }
    if lower.contains("not found")
        || lower.contains("no such file")
        || lower.contains("does not exist")
        || lower.contains("404")
    {
        return "not_found";
    }
    if lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("forbidden")
        || lower.contains("401")
        || lower.contains("403")
    {
        return "permission";
    }
    "validation"
}

/// Generic action guidance per failure type — used when no pattern matches.
pub(super) fn action_for_type(failure_type: &str) -> &'static str {
    match failure_type {
        "transient" => "RETRY: Transient failure — wait briefly, then retry the same command.",
        "not_found" => {
            "ADAPT: Resource not found — this is a valid result, not an error. Adjust approach."
        }
        "permission" => "ESCALATE: Access denied — try alternative tool or ask user for access.",
        _ => "DIAGNOSE: Read the error. FIX: Correct the cause. RETRY: Run again.",
    }
}

/// True iff the PREVIOUS unresolved failure marker matches this
/// (tool, failure-class) — the recurrence signal that gates Tier-2
/// `[SELF_EVOLVE]` emission. First occurrences seed the pattern store
/// silently: the raw tool error is already visible in the transcript, and
/// injecting a research directive for every one-off mistake (a mistyped
/// flag, an oversized read) was loop-derailing noise. A repeat means the
/// agent hit the same class twice without an intervening success — THAT is
/// worth a research detour. Markers reset via `clear_failure` on success.
pub(super) fn is_repeat_failure(
    last_tool: &str,
    last_type: &str,
    tool: &str,
    failure_type: &str,
) -> bool {
    !last_tool.is_empty() && last_tool == tool && last_type == failure_type
}
