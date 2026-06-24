use std::collections::HashMap;

use crate::state::SessionState;

/// Sentinel stored in `subagent_outputs` while an agent is still running.
/// A non-negative value means the agent finished (the value is its output size).
/// `active_subagents` is ALWAYS the count of entries equal to this sentinel —
/// never an independently-mutated scalar — so start/stop cannot drift it.
pub(crate) const SUBAGENT_RUNNING: i32 = -1;

impl SessionState {
    /// Recompute `active_subagents` from `subagent_outputs` (single source of
    /// truth). Called after every mutation so the cached scalar — which is what
    /// serializes and what the stop gate reads — can never disagree with the map.
    fn recount_active_subagents(&mut self) {
        let running = self
            .subagent_outputs
            .values()
            .filter(|&&v| v == SUBAGENT_RUNNING)
            .count();
        self.active_subagents = i32::try_from(running).unwrap_or(i32::MAX);
    }

    pub fn track_subagent_start(&mut self, agent_id: &str) {
        // Idempotent + drift-free: mark this id running. If the same id is
        // reused across runs (Claude Code recycles agent_ids), the previous
        // finished value is overwritten with the running sentinel, so the
        // recount correctly sees it as active again — the old contains_key
        // guard silently dropped this case, under-counting the live agent.
        self.subagent_outputs
            .insert(agent_id.into(), SUBAGENT_RUNNING);
        self.recount_active_subagents();
        self.last_subagent_turn = self.turn_count;
        self.save_or_log();
    }

    /// Subagents are stale if active > 0 but no activity for 2+ turns.
    /// `SubagentStop` may never fire (crash, timeout, hook failure),
    /// leaving a stale counter that blocks the stop gate forever.
    #[must_use]
    pub const fn has_stale_subagents(&self) -> bool {
        self.active_subagents > 0 && self.turn_count.saturating_sub(self.last_subagent_turn) >= 2
    }

    pub fn track_subagent_stop(&mut self, agent_id: &str, output_size: i32) {
        // Record the finished output size (>= 0 means done). Clamp any negative
        // size up to 0 so it can never collide with the SUBAGENT_RUNNING (-1)
        // sentinel and leave a finished agent counted as active.
        self.subagent_outputs
            .insert(agent_id.into(), output_size.max(0));
        // active_subagents is derived from the map, so a stop with no matching
        // start (crash, prior-session start, recycled id) simply records a
        // finished entry — it can no longer push the counter below zero or
        // decrement a count that belongs to a different live agent.
        self.recount_active_subagents();
        // Large outputs (>2000 chars) from Explore/research agents likely contain
        // actionable findings. Set pending flag so stop gate enforces action.
        if output_size >= 2000 {
            self.subagent_action_pending = true;
            self.subagent_action_turn = self.turn_count;
        }
        self.save_or_log();
    }

    /// Mark subagent action as resolved (parent acted on findings).
    /// Called by post-write gate when code is written after subagent returned.
    pub fn clear_subagent_action(&mut self) {
        self.subagent_action_pending = false;
        self.subagent_action_turn = 0;
        self.save_or_log();
    }

    /// Check if a subagent returned actionable results that haven't been acted on.
    /// Returns true if action is pending AND it's been at least 1 turn since.
    #[must_use]
    pub const fn has_unacted_subagent_result(&self) -> bool {
        self.subagent_action_pending && self.turn_count > self.subagent_action_turn
    }

    /// Get cumulative blast radius stats for gate decisions.
    #[must_use]
    pub const fn get_blast_stats(&self) -> (usize, usize, bool) {
        (
            self.subagent_files_written.len(),
            self.subagent_external_apis.len(),
            self.subagent_db_mutations,
        )
    }

    /// Check if blast has escalated (threshold exceeded).
    #[must_use]
    pub const fn is_blast_escalated(&self) -> bool {
        self.blast_escalated
    }

    /// Get denied tools list for injection into subagent context.
    #[must_use]
    pub fn get_denied_tools_context(&self) -> String {
        if self.subagent_denied_tools.is_empty() {
            return String::new();
        }
        format!(
            "[INHERITED_DENIED_TOOLS]\ntools: {}\nreason: Parent context denied these tools; subagent inherits restriction.\n",
            self.subagent_denied_tools.join(", ")
        )
    }

    #[must_use]
    pub fn get_effective_output_limit(
        &self,
        agent_type: &str,
        limits: &HashMap<String, i32>,
    ) -> i32 {
        let base = limits
            .get(agent_type)
            .copied()
            .unwrap_or(self.subagent_max_output);
        let (num, denom): (i64, i64) = match self.context_phase.as_str() {
            "mid" => (3, 4),
            "late" => (1, 2),
            "critical" => (1, 4),
            _ => (1, 1),
        };
        #[expect(
            clippy::integer_division,
            clippy::arithmetic_side_effects,
            reason = "denom is a literal from the closed context_phase match (1/2/4); never zero"
        )]
        let scaled = i64::from(base).saturating_mul(num) / denom;
        i32::try_from(scaled).unwrap_or(if scaled < 0 { i32::MIN } else { i32::MAX })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::state::SessionState;

    #[test]
    fn test_subagent_tracking() {
        let mut s = SessionState::default();
        s.track_subagent_start("agent-1");
        assert_eq!(s.active_subagents, 1);
        s.track_subagent_stop("agent-1", 5000);
        assert_eq!(s.active_subagents, 0);
        assert_eq!(s.subagent_outputs.get("agent-1"), Some(&5000));
    }

    #[test]
    fn test_subagent_action_pending_on_large_output() {
        let mut s = SessionState::default();
        s.turn_count = 3;
        s.track_subagent_start("explore-1");
        s.track_subagent_stop("explore-1", 5000); // >2000 = actionable
        assert!(s.subagent_action_pending);
        assert_eq!(s.subagent_action_turn, 3);
    }

    #[test]
    fn test_subagent_action_not_pending_on_small_output() {
        let mut s = SessionState::default();
        s.turn_count = 3;
        s.track_subagent_start("haiku-1");
        s.track_subagent_stop("haiku-1", 500); // <2000 = not actionable
        assert!(!s.subagent_action_pending);
    }

    #[test]
    fn test_has_unacted_subagent_result() {
        let mut s = SessionState::default();
        s.turn_count = 3;
        s.track_subagent_start("explore-1");
        s.track_subagent_stop("explore-1", 5000);
        assert!(!s.has_unacted_subagent_result()); // same turn — not yet
        s.turn_count = 4;
        assert!(s.has_unacted_subagent_result()); // next turn — pending
        s.clear_subagent_action();
        assert!(!s.has_unacted_subagent_result()); // cleared
    }

    #[test]
    fn test_recycled_agent_id_restart_recounts_active() {
        // Claude Code recycles agent_ids. The old contains_key guard silently
        // dropped a re-start, under-counting the live agent. Now a restart of a
        // finished id must mark it running again and recount to 1.
        let mut s = SessionState::default();
        s.track_subagent_start("agent-1");
        s.track_subagent_stop("agent-1", 5000);
        assert_eq!(s.active_subagents, 0);
        s.track_subagent_start("agent-1"); // recycled id, fresh run
        assert_eq!(s.active_subagents, 1);
        assert_eq!(
            s.subagent_outputs.get("agent-1"),
            Some(&super::SUBAGENT_RUNNING)
        );
    }

    #[test]
    fn test_stop_without_start_cannot_drift_below_zero() {
        // A SubagentStop with no matching start (crash, prior-session start,
        // recycled id) must not push the derived count negative — the old
        // always-decrement code did exactly that.
        let mut s = SessionState::default();
        s.track_subagent_stop("ghost", 5000);
        assert_eq!(s.active_subagents, 0);
        assert_eq!(s.subagent_outputs.get("ghost"), Some(&5000));
    }

    #[test]
    fn test_two_concurrent_agents_counted_independently() {
        let mut s = SessionState::default();
        s.track_subagent_start("a");
        s.track_subagent_start("b");
        assert_eq!(s.active_subagents, 2);
        s.track_subagent_stop("a", 100);
        assert_eq!(s.active_subagents, 1);
        s.track_subagent_stop("b", 100);
        assert_eq!(s.active_subagents, 0);
    }

    #[test]
    fn test_negative_output_size_clamped_not_treated_as_running() {
        // A negative output_size must not collide with SUBAGENT_RUNNING (-1) and
        // leave a finished agent counted as active.
        let mut s = SessionState::default();
        s.track_subagent_start("agent-1");
        s.track_subagent_stop("agent-1", -7);
        assert_eq!(s.active_subagents, 0);
        assert_eq!(s.subagent_outputs.get("agent-1"), Some(&0));
    }

    #[test]
    fn test_effective_output_limit() {
        let s = SessionState {
            context_phase: "mid".into(),
            subagent_max_output: 8000,
            ..Default::default()
        };
        let limits = HashMap::new();
        assert_eq!(s.get_effective_output_limit("test", &limits), 6000);
    }
}
