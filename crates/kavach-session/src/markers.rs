use crate::paths::now_datetime;
use crate::state::SessionState;

fn unix_seconds_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

impl SessionState {
    pub fn mark_research_done(&mut self) {
        self.research_done = true;
        self.save_or_log();
    }

    pub fn mark_research_done_with_topic(&mut self, topic: &str) {
        self.research_done = true;
        if !topic.is_empty() && !self.research_topics.contains(&topic.to_owned()) {
            self.research_topics.push(topic.into());
        }
        self.save_or_log();
    }

    pub fn mark_memory_queried(&mut self) {
        self.memory_queried = true;
        self.save_or_log();
    }

    /// Record that a code-reviewer agent completed against the current diff.
    /// Cures the `REVIEW_GATE` over-firing where the same warning re-fires on
    /// every Stop hook attempt despite multiple reviews already running.
    /// SOURCE: `decision:rca.review_gate_overfires`
    pub fn mark_review_completed(&mut self) {
        self.last_review_files_count = self.files_modified.len();
        self.last_review_at = unix_seconds_i64();
        self.save_or_log();
    }


    pub fn mark_post_compact(&mut self) {
        self.post_compact = true;
        self.compacted_at = now_datetime();
        self.compact_count = self.compact_count.saturating_add(1);
        self.save_or_log();
    }


    #[must_use]
    pub const fn is_post_compact(&self) -> bool {
        self.post_compact
    }

    pub fn increment_turn(&mut self) {
        self.turn_count = self.turn_count.saturating_add(1);
        self.save_or_log();
    }

    /// Stamp THIS turn as user-directed (called by the intent gate on every
    /// `UserPromptSubmit`). The stop gate reads `user_directive_turn == turn_count`
    /// to grant the user-focus override: a turn the user just steered must not be
    /// hijacked by the autonomous dispatcher onto a different kanban card.
    pub fn mark_user_directive(&mut self) {
        self.user_directive_turn = self.turn_count;
        self.save_or_log();
    }

    /// True iff the USER issued a directive on the CURRENT turn — the stop gate's
    /// user-focus override predicate (the user is STEERING; do not dispatch a
    /// different card over their live instruction).
    #[must_use]
    pub const fn user_is_steering_this_turn(&self) -> bool {
        self.user_directive_turn == self.turn_count && self.turn_count > 0
    }


    pub fn record_failure_typed(&mut self, tool: &str, failure_type: &str) {
        self.last_failure_tool = tool.into();
        self.last_failure_turn = self.turn_count;
        self.failure_type = failure_type.into();
        self.save_or_log();
    }

    pub fn clear_failure(&mut self) {
        self.last_failure_tool.clear();
        self.last_failure_turn = 0;
        self.failure_block_count = 0;
        self.failure_type.clear();
        self.save_or_log();
    }


    /// True if failure is a valid "not found" result, not a real error.
    #[must_use]
    pub fn is_not_found_failure(&self) -> bool {
        self.failure_type == "not_found"
    }

    /// Record a critical fact that must survive compaction.
    /// Capped at 20 entries. First 3 entries (founding facts) are preserved;
    /// eviction starts from position 3 to keep early session context intact.
    /// Newlines replaced with spaces to prevent serialization corruption.
    pub fn add_case_fact(&mut self, fact: &str) {
        const MAX_CASE_FACTS: usize = 20;
        const MAX_FACT_LEN: usize = 500;
        const PRESERVE_HEAD: usize = 3;
        if self.case_facts.len() >= MAX_CASE_FACTS {
            let evict_pos = PRESERVE_HEAD.min(self.case_facts.len().saturating_sub(1));
            self.case_facts.remove(evict_pos);
        }
        let sanitized: String = fact
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .take(MAX_FACT_LEN)
            .collect();
        self.case_facts.push(sanitized);
        self.save_or_log();
    }

    pub fn set_intent_risk(&mut self, risk: &str) {
        self.intent_risk = risk.into();
        self.save_or_log();
    }

    pub fn increment_files_read(&mut self) {
        self.subagent_files_read = self.subagent_files_read.saturating_add(1);
        self.save_or_log();
    }


    #[must_use]
    pub const fn has_recent_failure(&self) -> bool {
        !self.last_failure_tool.is_empty()
            && (self.last_failure_turn == self.turn_count || self.failure_block_count > 0)
    }

    /// Increment the stop-block counter for the current failure.
    /// Called by the stop gate each time it blocks due to an unresolved failure.
    pub fn increment_failure_blocks(&mut self) {
        self.failure_block_count = self
            .failure_block_count
            .saturating_add(1)
            .min(Self::max_failure_blocks().saturating_add(1));
        self.save_or_log();
    }

    /// Max number of times the stop gate will block before allowing forced stop.
    /// Gives Claude 3 chances to fix the failure before giving up.
    #[must_use]
    pub const fn max_failure_blocks() -> i32 {
        3
    }

    /// True iff the agent demonstrably made code-progress since the last
    /// Stop attempt — i.e. `last_write_turn` strictly advanced past the
    /// snapshot taken at the previous Stop. Multi-wave plans hit this on
    /// every wave (each wave writes code), so the breaker resets between
    /// waves instead of tripping after 3 waves. Live-lock safety preserved:
    /// a session with NO writes between Stops still trips the cap.
    /// SOURCE: AWS Builders' Library — "Avoiding fallback in distributed
    /// systems"; Martin Fowler — CircuitBreaker.html (success resets count).
    #[must_use]
    pub const fn has_progress_since_last_stop(&self) -> bool {
        self.last_write_turn > self.last_progress_snapshot_writes
            || self.last_db_write_turn > self.last_progress_snapshot_db_writes
    }

    /// Capture current progress signals into the snapshot fields. Called
    /// from `increment_stop_reblock` AFTER the progress check, so the next
    /// Stop attempt compares against THIS attempt's baseline.
    const fn snapshot_progress(&mut self) {
        self.last_progress_snapshot_turn = self.turn_count;
        self.last_progress_snapshot_writes = self.last_write_turn;
        self.last_progress_snapshot_db_writes = self.last_db_write_turn;
    }

    /// Increment the pending-work re-block counter (the bounded breaker for
    /// "kanban still has runnable work"). SEPARATE from
    /// `increment_failure_blocks`: this counter is NOT reset by
    /// `clear_failure()`, so a successful tool call between stop attempts
    /// cannot zero it. Without that separation the breaker never advanced
    /// past 1 and the stop gate looped forever.
    ///
    /// FIX [`state_drift`] rca.stop-breaker-no-progress-reset (2026-05-23):
    /// the breaker was counting Stop ATTEMPTS, so multi-wave plans hit cap=3
    /// after 3 waves regardless of forward progress. The reset edge
    /// (progress between Stops → counter to 0) was missing — the AWS/Fowler
    /// circuit-breaker pattern requires success to reset the count. Now:
    /// if `last_write_turn` advanced since the prior snapshot, the agent
    /// shipped code → reset to 0. Otherwise increment as before. Live-lock
    /// safety is preserved: 3 Stops with zero writes still trips the cap.
    // Plain save() RMW is correct here: Stop-hook-only single-writer path, no
    // concurrent same-session writer. SOURCE: decision.stop-reblock-rmw-single-writer-safe.
    pub fn increment_stop_reblock(&mut self) {
        if self.has_progress_since_last_stop() {
            self.stop_reblock_count = 0;
        } else {
            self.stop_reblock_count = self
                .stop_reblock_count
                .saturating_add(1)
                .min(Self::max_stop_reblocks().saturating_add(1));
        }
        self.snapshot_progress();
        self.save_or_log();
    }

    /// Clear the pending-work re-block counter. Called ONLY on a genuine
    /// clean stop (kanban empty / forced terminal), never on tool success.
    pub fn clear_stop_reblock(&mut self) {
        if self.stop_reblock_count != 0 {
            self.stop_reblock_count = 0;
            self.save_or_log();
        }
    }

    /// Max pending-work re-blocks before the stop gate stops re-blocking and
    /// allows the (forced) stop. The runnable card is NOT lost — it remains
    /// in the kanban for the next session; this only prevents an unbounded
    /// in-session spin on work the agent is not progressing.
    #[must_use]
    pub const fn max_stop_reblocks() -> i32 {
        3
    }
}

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;
