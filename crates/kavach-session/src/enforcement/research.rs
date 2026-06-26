use crate::state::SessionState;

impl SessionState {
    pub fn set_research_topic(&mut self, topic: &str) {
        self.research_topic = topic.into();
        self.save_or_log();
    }

    pub fn reset_evidence_window(&mut self) {
        self.websearch_count_since_intent = 0;
        self.intent_set_turn = self.turn_count;
        self.think_first_injected = false;
        self.research_advisory_sent = false;
        self.fanout_nudge_sent = false;
        self.files_modified_this_turn = Vec::new();
        self.tdd_red_units = Vec::new();
        self.rca_block_present = false;
        self.rca_set_turn = 0;
        self.save_or_log();
    }

    pub fn mark_rca_present(&mut self) {
        self.rca_block_present = true;
        self.rca_set_turn = self.turn_count;
        self.save_or_log();
    }

    #[must_use]
    pub const fn rca_satisfied(&self) -> bool {
        self.rca_block_present
    }

    pub fn record_websearch(&mut self) {
        self.websearch_count_since_intent = self.websearch_count_since_intent.saturating_add(1);
        self.save_or_log();
    }

    #[must_use]
    pub fn evidence_window_satisfied(&self) -> bool {
        if self.intent_type != "implement" {
            return true;
        }
        if self.websearch_count_since_intent > 0 {
            return true;
        }
        false
    }
}
