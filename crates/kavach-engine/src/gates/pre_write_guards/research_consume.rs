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
    let params = serde_json::json!({ "query": "local code analysis intent synonyms no external research", "limit": 8 });
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

/// Returns `Some(block_reason)` to DENY a research-required production write that
/// carries no source evidence — fail-closed internet-first enforcement. Drives the
/// Internet on the spot (kicks the background lookup) so the agent can satisfy the
/// gate immediately, then retry with a cited URL. `None` when research is satisfied,
/// not applicable, or bypassed — those paths never block.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    // Emergency escape hatch — disables enforcement entirely.
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
    // Local-analysis intents (canonical OR Brain-OS synonym) need no external lookup.
    if is_local_analysis_intent(session.intent_type.as_str()) {
        return None;
    }
    if is_comment_only(ctx.content) {
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

    // No evidence → BLOCK. Drive the Internet so the agent can cite + retry now.
    Some(block_for_missing_research(session))
}

/// Build the fail-closed block message. Reads the live research cache; kicks a fresh
/// background web search (`kavach_advisor::kickoff`) when none is running so findings
/// arrive fast; names exactly what unblocks the write (cite a URL / [RESEARCH] /
/// SOURCE:). The write is REFUSED on every branch — internet-first is a P0 LAW.
fn block_for_missing_research(session: &kavach_session::SessionState) -> String {
    let topic = session.research_topic.as_str();
    let sid = session.session_id.as_str();
    let tail = match kavach_advisor::read_findings(sid) {
        Some(f) if f.status == "done" => format!(
            "Internet-first lookup is COMPLETE for \"{topic}\" — cite a source URL \
             from these findings in the file or [RCA], then retry:\n{}",
            f.summary
        ),
        Some(f) if f.status == "pending" => format!(
            "Internet-first lookup is RUNNING for \"{topic}\". Wait for the findings \
             in the session cache, cite a source URL, then retry the write."
        ),
        _ => {
            kavach_advisor::kickoff(sid, topic);
            format!(
                "Launched an internet-first lookup for \"{topic}\". Cite a source URL \
                 once it lands, then retry the write."
            )
        }
    };
    format!(
        "[RESEARCH_FIRST:P0] BLOCKED. This turn requires research and this production \
         write cites NO source (no URL / [RESEARCH] / SOURCE: marker, and the research \
         cache is not done). No source -> no claim. {tail} If this looks wrong, READ this \
         guard's source and fix the real cause — never route around it."
    )
}

/// True when the live research cache for this session reports `done`.
fn cache_is_done(session_id: &str) -> bool {
    kavach_advisor::read_findings(session_id).is_some_and(|f| f.status == "done")
}

/// True when every non-blank changed line is a comment/attribute (no executable code).
fn is_comment_only(changed: &str) -> bool {
    let mut saw = false;
    for line in changed.lines() {
        let t = line.trim_start();
        if t.is_empty() {
            continue;
        }
        saw = true;
        if !(t.starts_with("//") || t.starts_with("#[") || t.starts_with("#!")) {
            return false;
        }
    }
    saw
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
