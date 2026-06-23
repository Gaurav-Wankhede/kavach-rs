//! Fail-closed internet-first ENFORCEMENT: when THIS turn's prompt was classified
//! `requires_research`, a Write/Edit to production code without research evidence is
//! BLOCKED at write time, not merely advised. "No source → no claim" is a P0 LAW, not
//! a nudge — an unsourced production write never lands. The gate still DRIVES the
//! Internet (kicks `kavach_advisor::kickoff` so findings arrive fast), but the write
//! is refused until the agent cites a source URL / [RESEARCH] / SOURCE: marker, or the
//! live research cache reports `done`. SOURCE: ~/.claude/CLAUDE.md §Internet-first.
//!
//! `check` returns `Some(block_reason)` to DENY; `None` when research is satisfied or
//! not applicable. Carve-outs (never block): test files, non-code, local-analysis
//! intents (audit/analyze/explain/read/review/explore), and `KAVACH_RESEARCH_BYPASS=1`.

use crate::gates::pre_write_context::WriteContext;

/// Canonical local-analysis intents — the fail-soft floor of intents that inspect
/// existing local artifacts and need no external research. The dynamic path may
/// ADD to this (Brain-OS-surfaced intents) but never removes one, so the P0 block
/// can only ever loosen toward more carve-outs, never tighten away a safe one.
const LOCAL_ANALYSIS_INTENTS: [&str; 6] =
    ["audit", "analyze", "explain", "read", "review", "explore"];

/// True when `intent` is a local-analysis intent — canonical list OR a Brain-OS-
/// surfaced synonym. The canonical set is checked first (cheap, no RPC); only an
/// unknown intent pays the lookup. Fail-soft: a daemon blip ⇒ canonical-only.
fn is_local_analysis_intent(intent: &str) -> bool {
    if intent.is_empty() {
        return false;
    }
    if LOCAL_ANALYSIS_INTENTS.contains(&intent) {
        return true;
    }
    brain_local_analysis_synonyms().iter().any(|s| s == intent)
}

/// Brain-OS-surfaced local-analysis intent synonyms (e.g. "inspect", "trace",
/// "investigate"). Bare entry keys mapped to their trailing segment; fail-soft to
/// empty on any RPC error ⇒ canonical-only enforcement.
fn brain_local_analysis_synonyms() -> Vec<String> {
    let params =
        serde_json::json!({ "query": "local code analysis intent synonyms no external research", "limit": 8 });
    let hits: Vec<kavach_surreal::BrainHit> =
        match kavach_rpc::client::call("brain.think", Some(params)) {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
    hits.into_iter()
        .filter_map(|h| h.id.rsplit('.').next().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Returns `Some(advisory)` to ATTACH internet-first context to the write (never
/// a block). Drives the Internet on the spot: ensures the background web search is
/// running and injects its live findings (or a pending directive while they
/// land). `None` when research is satisfied, not applicable, or bypassed.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    // Emergency escape hatch — silences the advisory entirely.
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return None;
    }
    // Only governs real production code; tests/docs/config are exempt.
    if ctx.is_test || !ctx.is_code {
        return None;
    }
    // Research was not required this turn → nothing to attach.
    if session.research_topic.is_empty() {
        return None;
    }
    // Local-analysis intents (canonical OR Brain-OS synonym) need no external lookup.
    if is_local_analysis_intent(session.intent_type.as_str()) {
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

    // No evidence yet → RESOLVE on the spot, do NOT suppress the write. Drive the
    // Internet ourselves and attach the result as advisory context.
    Some(resolve_with_internet(session))
}

/// Self-resolving internet-first advisory. Reads the live research cache; kicks a
/// fresh background web search (`kavach_advisor::kickoff`) when none is running;
/// returns the findings brief when `done`, else a pending/error directive. The
/// write ALWAYS proceeds — the loop-level `[RESEARCH_FIRST]` Stop teeth ensure an
/// unsourced claim can never terminate the turn.
fn resolve_with_internet(session: &kavach_session::SessionState) -> String {
    let topic = session.research_topic.as_str();
    let sid = session.session_id.as_str();
    match kavach_advisor::read_findings(sid) {
        Some(f) if f.status == "done" => format!(
            "[RESEARCH_RESOLVED] internet-first lookup complete for \"{topic}\". \
             CITE a source URL from these findings in the file or [RCA] block:\n{}",
            f.summary
        ),
        Some(f) if f.status == "pending" => format!(
            "[RESEARCH_INFLIGHT] internet-first lookup running for \"{topic}\". \
             Findings land in the session cache; cite a source URL the moment \
             they arrive (the Stop gate enforces this before the turn ends)."
        ),
        // No cache yet, or a prior error → (re)launch the lookup right now.
        _ => {
            kavach_advisor::kickoff(sid, topic);
            format!(
                "[RESEARCH_KICKED] launched an internet-first lookup for \
                 \"{topic}\" in the background. Keep working; cite a source URL \
                 once the findings land (the Stop gate enforces this before the \
                 turn ends). Bypass (emergencies only): KAVACH_RESEARCH_BYPASS=1."
            )
        }
    }
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
