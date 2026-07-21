//! Fail-closed internet-first ENFORCEMENT: when THIS turn's prompt was classified
//! `requires_research`, a Write/Edit to production code without research evidence is
//! BLOCKED at write time, not merely advised. "No source → no claim" is a P0 LAW, not
//! a nudge — an unsourced production write never lands. The gate still DRIVES the
//! Internet (kicks `kavach_advisor::kickoff` so findings arrive fast), but the write
//! is refused until the agent cites a source URL / [RESEARCH] / SOURCE: marker, or the
//! live research cache reports `done`. SOURCE: ~/.claude/CLAUDE.md §Internet-first.
//!
//! CIRCUIT BREAKER: after `gate_circuit_breaker_threshold` (default 3) blocks on the
//! same file, the gate force-allows the write and records a `mistake` so the loop
//! does not spin forever. The surrender is NEVER silent — audited and surfaced.
//!
//! `check` returns `Some(block_reason)` to DENY; `None` when research is satisfied,
//! not applicable, bypassed, or the circuit breaker has tripped — loop-safety override.
use crate::gates::pre_write_context::WriteContext;

const LOCAL_ANALYSIS_INTENTS: [&str; 6] =
    ["audit", "analyze", "explain", "read", "review", "explore"];

fn is_local_analysis_intent(intent: &str) -> bool {
    if intent.is_empty() {
        return false;
    }
    if LOCAL_ANALYSIS_INTENTS.contains(&intent) {
        return true;
    }
    brain_local_analysis_synonyms().iter().any(|s| s == intent)
}

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

/// Returns `Some(block_reason)` to DENY; `None` when satisfied, not applicable,
/// bypassed, or the per-file circuit breaker has tripped.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    session: &mut kavach_session::SessionState,
) -> Option<String> {
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return None;
    }
    if ctx.is_test || !ctx.is_code {
        return None;
    }
    if session.research_topic.is_empty() {
        return None;
    }
    if is_local_analysis_intent(session.intent_type.as_str()) {
        return None;
    }
    if is_comment_only(ctx.content) {
        return None;
    }
    if session.research_done && cache_is_done(&session.session_id) {
        return None;
    }
    if content_has_evidence(&ctx.effective_content) {
        return None;
    }

    // CIRCUIT BREAKER: per-file key so cross-file work doesn't pollute counts.
    let file_key = format!("research:{}", ctx.file_path);
    if session.is_gate_tripped(&file_key) {
        let banned = format!(
            "research gate tripped after {} blocks on {}",
            session.gate_block_count(&file_key),
            ctx.file_path
        );
        let turn = session.turn_count;
        drop(kavach_session::record_mistake_surfaced(
            session,
            "research_circuit_breaker_tripped",
            &banned,
            "Force-allowed write after repeated research blocks; agent failed to cite source",
            turn,
        ));
        return None;
    }

    // Record block and emit denial.
    session.record_gate_block(&file_key);
    Some(block_for_missing_research(session))
}

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
        "[RESEARCH_EVIDENCE] Add ONE line above the change: // SOURCE: <url-you-read> — one \
         line satisfies BOTH this gate and the one-line comment ceiling (no conflict exists) \
         -> RETRY this write. Alternatives: [RESEARCH] block in your reply, or finish the \
         pending lookup. {tail}"
    )
}

fn cache_is_done(session_id: &str) -> bool {
    kavach_advisor::read_findings(session_id).is_some_and(|f| f.status == "done")
}

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

fn content_has_evidence(content: &str) -> bool {
    content.contains("http://")
        || content.contains("https://")
        || content.contains("[RESEARCH]")
        || content.contains("research(")
        || content.contains("SOURCE:")
}

#[cfg(test)]
#[path = "research_consume_test.rs"]
mod tests;
