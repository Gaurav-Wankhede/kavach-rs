//! `ToolChainValidator` (P1-B): scores tool-call coherence against the stated
//! intent. P1-A gates check each tool in isolation; P1-B looks at the
//! sequence and flags intent↔chain mismatches.
//!
//! Severity: `P1Advisory`. Host emits `[TOOL_CHAIN_MISMATCH]` context.
//! SOURCE: kavach-engine/CLAUDE.md Gate Severity Policy

// ALGO: vec_suffix_scan
// PROBLEM_CLASS: bounded-FIFO + read-recent-N (N ≤ 10)
// REJECTED: [
//   {"name":"VecDeque","reason":"eviction adds correctness surface; perf gain immeasurable at N=10"},
//   {"name":"SmallVec","reason":"workspace dep for ~80B inline savings; not justified"}
// ]
// TIME: O(N) read | SPACE: O(N) caller-bounded
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: caller owns the bound (no auto-eviction); acceptable because session
//   state already enforces a recent-tools cap upstream.
// BENCHMARK: https://nnethercote.github.io/perf-book/data-structures.html
// SOURCE: https://doc.rust-lang.org/std/collections/struct.VecDeque.html#performance

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainSeverity {
    P1Advisory,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChainHit {
    pub severity: ChainSeverity,
    pub reason: &'static str,
    pub fix: &'static str,
}

const RESEARCH_WRITE_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];
const READ_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
const WRITE_TOOLS: &[&str] = &["Edit", "Write"];
const DEBUG_EDITS_THRESHOLD: usize = 3;
const IMPLEMENT_EDITS_THRESHOLD: usize = 3;
const READ_LOOKBACK: usize = 5;

#[must_use]
pub fn validate(
    intent_type: &str,
    recent_tools: &[String],
    current_tool: &str,
) -> Option<ChainHit> {
    if intent_type == "research" && RESEARCH_WRITE_TOOLS.contains(&current_tool) {
        return Some(ChainHit {
            severity: ChainSeverity::P1Advisory,
            reason: "intent=research but current tool mutates files",
            fix: "Research is read-only by contract. Switch intent to implement/debug if writes are needed.",
        });
    }
    if intent_type == "debug" && current_tool == "Edit" {
        let trailing_edits = recent_tools
            .iter()
            .rev()
            .take_while(|t| WRITE_TOOLS.contains(&t.as_str()))
            .count();
        let any_read = recent_tools
            .iter()
            .rev()
            .take(READ_LOOKBACK)
            .any(|t| READ_TOOLS.contains(&t.as_str()));
        if trailing_edits >= DEBUG_EDITS_THRESHOLD && !any_read {
            return Some(ChainHit {
                severity: ChainSeverity::P1Advisory,
                reason: "debug intent: 3+ consecutive Edits and no recent Read",
                fix: "Re-read the failing surface (test output, error log) before editing again.",
            });
        }
    }
    if intent_type == "implement" && current_tool == "WebSearch" {
        let recent_edits = recent_tools
            .iter()
            .rev()
            .take(READ_LOOKBACK)
            .filter(|t| WRITE_TOOLS.contains(&t.as_str()))
            .count();
        if recent_edits >= IMPLEMENT_EDITS_THRESHOLD {
            return Some(ChainHit {
                severity: ChainSeverity::P1Advisory,
                reason: "WebSearch mid-implement after 3+ edits — likely drift",
                fix: "Complete the in-flight edit cluster first OR commit a [REPLAN] note.",
            });
        }
    }
    None
}

#[cfg(test)]
#[path = "tool_chain_validator_tests.rs"]
mod tests;
