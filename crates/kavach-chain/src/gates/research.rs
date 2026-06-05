use crate::chain_state::ChainState;
use crate::types::{IntentAnalysis, ResearchStatus, VerificationResult};
use std::collections::HashMap;

use chrono::Local;

/// Consult the process-wide loader (scanned once at first use) and return
/// `is_research_class()` for `agent_type`. Returns false on missing dir, missing
/// file, parse error — safe-fail: gate falls through to normal tabula-rasa
/// path. SOURCE: `decision:rca.agent_routing_gate_awareness`.
fn agent_is_research_class(agent_type: &str) -> bool {
    if agent_type.is_empty() {
        return false;
    }
    crate::loader::global_loader()
        .and_then(|l| l.get_agent(agent_type))
        .is_some_and(|a| a.is_research_class())
}

// ALGO: HashSet O(1) membership check for session-scope gate satisfaction
// PROBLEM_CLASS: idempotent-gate short-circuit (one bool check per gate per turn)
// REJECTED: [
//   {"name":"per-turn re-eval (status quo)","reason":"verified FP storm — gate re-fires every turn despite prior satisfaction; chain_*.json evidence in this session"},
//   {"name":"timestamp-based TTL","reason":"adds clock dependency; satisfaction is session-scoped not time-scoped per CLAUDE.md §13"},
//   {"name":"tag-by-topic-hash","reason":"correct long-term but defers fix; first-cut uses gate-name only, accepting cross-topic carry-over as documented tradeoff"}
// ]
// TIME: O(1) HashSet contains
// SPACE: O(N) where N≤~10 gates per session
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: gate-name-only key means switching topic mid-session keeps satisfaction; acceptable per "research-once-per-session" contract documented in decision:rca.gate_session_amnesia
// BENCHMARK: https://nnethercote.github.io/perf-book/hashing.html (std HashSet ~20ns; dwarfed by file I/O elsewhere in chain)
// SOURCE: https://doc.rust-lang.org/std/collections/struct.HashSet.html#method.contains

pub(crate) fn run_gate(
    state: &mut ChainState,
    research_done: bool,
    prompt: &str,
    agent_type: &str,
) {
    // Frontmatter-driven research-class bypass: if the active agent is
    // provably read-only AND research-capable per its own ~/.claude/agents/*.md
    // contract, blocking it would be double-gating. Trust comes from the agent
    // file itself, NOT from a const list. See decision:rca.agent_routing_gate_awareness.
    if agent_is_research_class(agent_type) {
        let mut r = VerificationResult {
            gate: "RESEARCH".into(),
            status: "pass".into(),
            reason: format!(
                "Research-class agent ({agent_type}): tabula-rasa satisfied by frontmatter contract"
            ),
            context: HashMap::new(),
            timestamp: String::new(),
            next_action: String::new(),
        };
        r.context.insert("agent_type".into(), agent_type.into());
        r.context.insert("research_class".into(), "true".into());
        state.add_result(r);
        state.mark_satisfied("RESEARCH");
        return;
    }

    // Session-scope short-circuit: skip re-litigation if RESEARCH was satisfied
    // earlier this session. See decision:rca.gate_session_amnesia.
    if state.is_satisfied("RESEARCH") {
        let mut r = VerificationResult {
            gate: "RESEARCH".into(),
            status: "pass".into(),
            reason: "Session-satisfied: research completed earlier this session".into(),
            context: HashMap::new(),
            timestamp: String::new(),
            next_action: String::new(),
        };
        r.context.insert("session_satisfied".into(), "true".into());
        state.add_result(r);
        return;
    }

    let research = research_check(state.intent.as_ref(), research_done, prompt);

    let mut result = VerificationResult {
        gate: "RESEARCH".into(),
        status: "pass".into(),
        reason: "TABULA_RASA compliance verified".into(),
        context: HashMap::new(),
        timestamp: String::new(),
        next_action: String::new(),
    };

    if research.bypass {
        result.reason = format!("Bypassed: {}", research.bypass_reason);
        // Mark BEFORE add_result so a panic between the two cannot leave the
        // result recorded but satisfaction lost (next turn would re-fire).
        state.mark_satisfied("RESEARCH");
        state.research = Some(research);
        state.add_result(result);
        return;
    }

    let requires = state.intent.as_ref().is_some_and(|i| i.requires_research);
    if !research.done && requires {
        result.status = "block".into();
        let itype = state
            .intent
            .as_ref()
            .map_or("unknown", |i| i.intent_type.as_str());
        result.reason = format!("TABULA_RASA: Research required before {itype}");
        if !research.suggested_query.is_empty() {
            result.next_action = format!("WebSearch: {}", research.suggested_query);
            result
                .context
                .insert("suggested_query".into(), research.suggested_query.clone());
        }
    } else {
        // Pass path: mark BEFORE add_result for the same atomicity reason.
        state.mark_satisfied("RESEARCH");
    }

    state.research = Some(research);
    state.add_result(result);
}

#[must_use]
pub fn research_check(
    intent: Option<&IntentAnalysis>,
    research_done: bool,
    prompt: &str,
) -> ResearchStatus {
    let mut status = ResearchStatus {
        done: research_done,
        sources: Vec::new(),
        suggested_query: String::new(),
        bypass: false,
        bypass_reason: String::new(),
    };

    let lower = prompt.to_lowercase();
    let bypass_patterns = [
        "typo",
        "comment",
        "rename",
        "format",
        "whitespace",
        "spacing",
        "fix typo",
    ];
    for p in &bypass_patterns {
        if lower.contains(p) {
            status.bypass = true;
            status.bypass_reason = format!("Trivial change: {p}");
            return status;
        }
    }

    // The prompt-keyword bypass above keys off USER PROMPT TEXT, not the edit
    // itself — so a short confirmation reply ("yes", "go ahead") that authorizes
    // a comment-only edit is invisible to it. When the classifier falls back to
    // the generic `general` intent, it must NOT force research: `general` is the
    // catch-all bucket (typically confidence ~0.5) and carries no evidence that
    // the work is research-class. A low-confidence catch-all classification
    // should route to a fallback, not a hard gate. Treat it as a soft bypass to
    // stop false-positive TABULA_RASA blocks on trivial follow-up edits.
    // SOURCE: decision:rca.research_gate_general_intent_false_positive.
    if let Some(intent) = intent
        && intent.intent_type == "general"
    {
        status.bypass = true;
        status.bypass_reason = "Generic `general` intent — no research-class evidence".into();
        return status;
    }

    if let Some(intent) = intent
        && intent.requires_research
        && !research_done
    {
        status.done = false;
        status.suggested_query = build_search_query(&intent.intent_type, prompt);
    }

    status
}

fn build_search_query(intent_type: &str, prompt: &str) -> String {
    let year = Local::now().format("%Y").to_string();
    let stop = ["this", "that", "with", "from", "have", "been"];
    let kw: String = prompt
        .split_whitespace()
        .filter(|w| w.len() > 3 && !stop.contains(w))
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    match intent_type {
        "implement" => format!("{kw} implementation patterns {year} best practices"),
        "debug" => format!("{kw} debugging troubleshooting {year} root cause"),
        "security" => format!("{kw} security best practices {year} OWASP"),
        "deploy" => format!("{kw} deployment patterns {year} production"),
        "refactor" => format!("{kw} refactoring patterns {year} clean code"),
        "memory" => format!("{kw} persistent memory architecture {year}"),
        _ => format!("{kw} latest patterns {year}"),
    }
}

#[cfg(test)]
#[path = "research_tests.rs"]
mod tests;
