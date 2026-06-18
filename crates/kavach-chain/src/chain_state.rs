use std::collections::HashSet;

use chrono::Local;

use crate::types::{
    AegisVerification, CEODecision, IntentAnalysis, ResearchStatus, VerificationResult,
};

//   {"name":"Vec<String> + .contains","reason":"O(N) lookup per gate-check; N small enough to be fine but HashSet is idiomatic and read-dominated"},
//   {"name":"BTreeSet<String>","reason":"O(log N) ordered lookup unneeded; we don't iterate in order; HashSet's O(1) average wins"},
//   {"name":"HashSet<&'static str>","reason":"would force gate-name allowlist at compile-time; we want runtime extensibility per §13 inviolable"}
// ]
// TIME: O(1) average insert/lookup
// SPACE: O(N) where N≤~10 gates
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://doc.rust-lang.org/std/collections/struct.HashSet.html
//
// Cures session-scope amnesia. See decision:rca.gate_session_amnesia and
// ~/.claude/CLAUDE.md §13. #[serde(default)] keeps backward-compat with
// existing chain_*.json files written before satisfied_gates existed.
// SOURCE: https://serde.rs/field-attrs.html#default
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at ChainState::new handler boundary, exhaustively matched cross-crate"
)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainState {
    pub session_id: String,
    pub intent: Option<IntentAnalysis>,
    pub ceo: Option<CEODecision>,
    pub aegis: Option<AegisVerification>,
    pub research: Option<ResearchStatus>,
    pub results: Vec<VerificationResult>,
    pub final_status: String,
    #[serde(default)]
    pub satisfied_gates: HashSet<String>,
    /// Per-agent count of how many times the router has SUGGESTED this agent
    /// in the current session. Once count >= N, suppress further suggestions —
    /// user clearly hasn't delegated, stop nagging.
    /// SOURCE: `decision:rca.agent_routing_token_cost`
    /// SOURCE: <https://users.rust-lang.org/t/counter-based-on-hashmap/96225> (entry API counter pattern)
    #[serde(default)]
    pub suggestion_counts: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub kernel_observed: Option<String>,
}

impl ChainState {
    #[must_use]
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.into(),
            intent: None,
            ceo: None,
            aegis: None,
            research: None,
            results: Vec::new(),
            final_status: "pending".into(),
            satisfied_gates: HashSet::new(),
            suggestion_counts: std::collections::HashMap::new(),
            kernel_observed: None,
        }
    }

    /// Record that the router suggested `agent_name` this turn. Returns the
    /// new count for the agent (post-increment). Single-lookup entry pattern.
    pub fn record_suggestion(&mut self, agent_name: &str) -> u32 {
        let entry = self
            .suggestion_counts
            .entry(agent_name.to_owned())
            .or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    }

    /// True iff the router has already suggested `agent_name` >= `threshold`
    /// times this session — caller should suppress further suggestions.
    #[must_use]
    pub fn is_suggestion_saturated(&self, agent_name: &str, threshold: u32) -> bool {
        self.suggestion_counts
            .get(agent_name)
            .is_some_and(|c| *c >= threshold)
    }

    pub fn add_result(&mut self, mut result: VerificationResult) {
        result.timestamp = Local::now().to_rfc3339();
        if result.status == "block" {
            self.final_status = "blocked".into();
        }
        self.results.push(result);
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.final_status == "blocked"
    }

    #[must_use]
    pub fn get_block_reason(&self) -> String {
        for r in &self.results {
            if r.status == "block" {
                return format!("{}: {}", r.gate, r.reason);
            }
        }
        String::new()
    }

    /// Mark a gate as satisfied for the remainder of this session.
    pub fn mark_satisfied(&mut self, gate: &str) {
        self.satisfied_gates.insert(gate.to_owned());
    }

    /// Check whether a gate has already been satisfied this session.
    #[must_use]
    pub fn is_satisfied(&self, gate: &str) -> bool {
        self.satisfied_gates.contains(gate)
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
