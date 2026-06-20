//! P0 enforced-consume gate: when THIS turn's prompt was classified
//! `requires_research`, a Write/Edit to production code is BLOCKED until the
//! turn presents research evidence — a source URL, an `[RESEARCH]` block, or a
//! completed live research-cache entry. This is the teeth of the internet-first
//! policy: the intent gate fires the lookup; this gate refuses code that ignored
//! it.
//!
//! Fail-safe: the default on any ambiguity is to BLOCK (no evidence ⇒ no write).
//! Escape hatch: `KAVACH_RESEARCH_BYPASS=1` for emergencies. Carve-outs mirror
//! the `pre_tool` research gate (local-analysis intents, test files).

use crate::gates::pre_write_context::WriteContext;

/// Intents that inspect existing local artifacts and need no external research.
const LOCAL_ANALYSIS_INTENTS: [&str; 6] =
    ["audit", "analyze", "explain", "read", "review", "explore"];

/// Returns `Some(reason)` to BLOCK the write when research was required this turn
/// and no evidence of it is present.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    // Emergency escape hatch.
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return None;
    }
    // Only governs real production code; tests/docs/config are exempt.
    if ctx.is_test || !ctx.is_code {
        return None;
    }
    // Research was not required this turn → nothing to enforce.
    if session.research_topic.is_empty() {
        return None;
    }
    // Local-analysis intents inspect existing code; no external lookup needed.
    if LOCAL_ANALYSIS_INTENTS.contains(&session.intent_type.as_str()) {
        return None;
    }
    // Evidence path 1: the agent self-marked research done AND a live cache entry
    // confirms a completed lookup (self-attestation alone is not enough).
    if session.research_done && cache_is_done(&session.session_id) {
        return None;
    }
    // Evidence path 2: this write itself cites a source URL or a research block —
    // the agent did the lookup and is recording it inline.
    if content_has_evidence(&ctx.effective_content) {
        return None;
    }

    let topic = session.research_topic.as_str();
    Some(format!(
        "[RESEARCH_REQUIRED:P0] BLOCKED. This turn was classified \
         requires_research (topic: \"{topic}\") and you are writing production \
         code with NO evidence of an internet-first lookup.\n\
         DO THIS NOW: WebSearch \"{topic}\", corroborate across 2+ current \
         sources, then either (a) cite a source URL in the file/[RCA] block, or \
         (b) record an [RESEARCH] block this turn. The pre-write gate re-checks \
         and clears once evidence is present. Bypass (emergencies only): \
         KAVACH_RESEARCH_BYPASS=1."
    ))
}

/// True when the live research cache for this session reports `done`.
fn cache_is_done(session_id: &str) -> bool {
    kavach_advisor::read_findings(session_id).is_some_and(|f| f.status == "done")
}

/// True when the content cites a source URL or carries a research/RCA marker.
fn content_has_evidence(content: &str) -> bool {
    content.contains("http://")
        || content.contains("https://")
        || content.contains("[RESEARCH]")
        || content.contains("research(")
        || content.contains("SOURCE:")
}

#[cfg(test)]
mod tests;
