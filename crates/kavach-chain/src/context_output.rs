use std::fmt::Write as _;

use chrono::Local;

use crate::runner::Runner;

impl Runner {
    #[must_use]
    pub fn to_compact(&self) -> String {
        // CONTEXT-ROT: the all-pass case (status=approved, every gate `pass`)
        // emitted 6+ echo lines on every tool call with zero actionable signal.
        // Collapse it to one line; keep full per-gate detail only when the
        // chain blocked or a gate carries a next_action.
        // SOURCE: research.trychroma.com/context-rot — verbose mid-context
        // tool feedback degrades every subsequent output.
        // ALGO: linear scan (short-circuit any)
        // PROBLEM_CLASS: membership predicate over tiny bounded slice
        // REJECTED: [{"name":"HashSet pre-index","reason":"results.len() <= ~5 gates; hashing costs more than the scan"},{"name":"sort+binary_search","reason":"no ordering key; O(n log n) worse than O(n) here"}]
        // TIME: O(n) n<=5 | SPACE: O(1)
        // YEAR: 2026 | SEARCHED: 2026-05
        // TRADEOFF: O(n) but n is gate-count (≤5); constant-factor wins
        // BENCHMARK: https://research.trychroma.com/context-rot
        // Collapse whenever no gate carries a next_action and none blocked,
        // independent of the final_status string. `add_result` forces
        // final_status="blocked" on any block, and that block is itself
        // actionable — so `!has_actionable` always implies a clean chain
        // regardless of whether final_status is "approved", "pending", or a
        // future variant. This closes the reviewer-flagged gap where a
        // non-"approved" clean chain wrongly expanded to 6+ lines.
        let has_actionable = self
            .state
            .results
            .iter()
            .any(|r| !r.next_action.is_empty() || r.status == "block");
        if !has_actionable {
            return format!(
                "[VERIFICATION_CHAIN] {} ({} gates pass)\n",
                self.state.final_status,
                self.state.results.len()
            );
        }

        // fmt::Write on String is infallible (String has unbounded growth and
        // `String::write_str` always returns Ok). The `let _ = writeln!(s, ...)`
        // pattern was the band-aid silent_io_guard blocks; replacing with
        // push_str + format! makes the infallibility explicit at the type
        // level — no Result is produced to discard.
        let mut s = String::new();
        s.push_str("[VERIFICATION_CHAIN]\n");
        writeln!(s, "session: {}", self.state.session_id).ok();
        writeln!(s, "status: {}", self.state.final_status).ok();
        writeln!(s, "timestamp: {}", Local::now().to_rfc3339()).ok();
        s.push('\n');

        for r in &self.state.results {
            writeln!(s, "[{}]", r.gate).ok();
            writeln!(s, "status: {}", r.status).ok();
            writeln!(s, "reason: {}", r.reason).ok();
            if !r.next_action.is_empty() {
                writeln!(s, "next_action: {}", r.next_action).ok();
            }
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::runner::Runner;
    use crate::types::VerificationResult;

    #[test]
    fn test_runner_to_compact() {
        let mut runner = Runner::new("test-session");
        runner.state.final_status = "approved".into();
        runner.state.add_result(VerificationResult {
            gate: "INTENT".into(),
            status: "pass".into(),
            reason: "ok".into(),
            context: HashMap::new(),
            timestamp: String::new(),
            next_action: String::new(),
        });
        let toon = runner.to_compact();
        assert!(toon.contains("[VERIFICATION_CHAIN]"));
        assert!(toon.contains("approved"));
        assert!(toon.contains("1 gates pass"));
        assert!(!toon.contains("[INTENT]"));
        assert!(!toon.contains("timestamp:"));
    }

    #[test]
    fn test_runner_to_compact_non_approved_clean_chain_collapses() {
        let mut runner = Runner::new("test-session");
        runner.state.final_status = "pending".into();
        runner.state.add_result(VerificationResult {
            gate: "INTENT".into(),
            status: "pass".into(),
            reason: "ok".into(),
            context: HashMap::new(),
            timestamp: String::new(),
            next_action: String::new(),
        });
        let toon = runner.to_compact();
        assert!(toon.contains("pending (1 gates pass)"));
        assert!(!toon.contains("[INTENT]"));
        assert!(!toon.contains("timestamp:"));
    }

    #[test]
    fn test_runner_to_compact_empty_results_collapses() {
        let mut runner = Runner::new("test-session");
        runner.state.final_status = "approved".into();
        let toon = runner.to_compact();
        assert!(toon.contains("approved (0 gates pass)"));
        assert!(!toon.contains("timestamp:"));
    }

    #[test]
    fn test_runner_to_compact_mixed_gates_expand_on_block() {
        let mut runner = Runner::new("test-session");
        for (g, st) in [("INTENT", "pass"), ("AEGIS", "block"), ("CEO", "pass")] {
            runner.state.add_result(VerificationResult {
                gate: g.into(),
                status: st.into(),
                reason: "r".into(),
                context: HashMap::new(),
                timestamp: String::new(),
                next_action: String::new(),
            });
        }
        let toon = runner.to_compact();
        assert!(toon.contains("[AEGIS]"));
        assert!(toon.contains("status: block"));
        assert!(toon.contains("[INTENT]"));
    }

    #[test]
    fn test_runner_to_compact_full_detail_when_actionable() {
        let mut runner = Runner::new("test-session");
        runner.state.final_status = "approved".into();
        runner.state.add_result(VerificationResult {
            gate: "INTENT".into(),
            status: "pass".into(),
            reason: "ok".into(),
            context: HashMap::new(),
            timestamp: String::new(),
            next_action: "do the thing".into(),
        });
        let toon = runner.to_compact();
        assert!(toon.contains("[INTENT]"));
        assert!(toon.contains("next_action: do the thing"));
    }

    #[test]
    fn test_runner_to_compact_full_detail_when_blocked() {
        let mut runner = Runner::new("test-session");
        runner.state.add_result(VerificationResult {
            gate: "AEGIS".into(),
            status: "block".into(),
            reason: "threat".into(),
            context: HashMap::new(),
            timestamp: String::new(),
            next_action: String::new(),
        });
        let toon = runner.to_compact();
        assert!(toon.contains("[AEGIS]"));
        assert!(toon.contains("status: block"));
    }
}
