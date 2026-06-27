use crate::state::SessionState;
impl SessionState {
    pub fn reset_research_for_new_prompt(&mut self) {
        // Preserve research + skill state when task is active (plan execution).
        // This prevents the "invoke skill → next prompt clears it → blocked again"
        // friction loop during multi-prompt implementation work.
        // Only reset when NO active task — fresh prompts need fresh enforcement.
        let task_active = self.has_task() && self.task_status == "in_progress";
        if task_active {
            self.subagent_files_read = 0;
            self.save_or_log();
            return;
        }
        self.research_done = false;
        self.research_topics.clear();
        self.required_skills.clear();
        self.invoked_skills.clear();
        self.research_topic.clear();
        self.subagent_files_read = 0;
        self.save_or_log();
    }
    #[must_use]
    pub const fn needs_reinforcement(&self) -> bool {
        let threshold = if self.reinforce_every_n == 0 {
            15
        } else {
            self.reinforce_every_n
        };
        self.turn_count.saturating_sub(self.last_reinforce_turn) >= threshold
    }
    pub fn mark_reinforcement_done(&mut self) {
        self.last_reinforce_turn = self.turn_count;
        self.save_or_log();
    }
    pub fn set_model(&mut self, model_id: &str) {
        self.model_id = model_id.into();
        let cfg = kavach_config::ModelConfig::from_model_id(model_id);
        self.token_budget_total = cfg.usable_budget;
        self.update_context_phase(); // update_context_phase calls save()
    }
    pub fn update_context_phase(&mut self) {
        if self.token_budget_total <= 0 {
            let cfg = kavach_config::ModelConfig::from_model_id(&self.model_id);
            self.token_budget_total = cfg.usable_budget;
        }
        #[expect(
            clippy::float_arithmetic,
            reason = "context-fill ratio; total guarded > 0 immediately above, no integer alternative for a fractional ratio"
        )]
        let ratio = if self.token_budget_total > 0 {
            f64::from(self.token_budget_used) / f64::from(self.token_budget_total)
        } else {
            0.0
        };
        // No budget-driven throttle tiers ("critical"/"late" removed): the model
        // works at full fidelity at every fill level and Claude Code's auto-compact
        // reclaims context losslessly at the window boundary. Only the benign
        // injection-depth tiers remain. SOURCE: decision.remove-context-budget-caps.
        self.context_phase = if ratio >= 0.3 { "mid" } else { "early" }.into();
        self.save_or_log();
    }
}
#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;
