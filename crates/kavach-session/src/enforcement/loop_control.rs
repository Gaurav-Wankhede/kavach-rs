use crate::state::SessionState;

impl SessionState {
    pub fn start_loop(&mut self, target: &str) {
        self.loop_active = true;
        self.loop_target = target.into();
        self.loop_iteration = 0;
        self.loop_start_turn = self.turn_count;
        self.save_or_log();
    }

    pub fn increment_loop(&mut self) {
        self.loop_iteration = self.loop_iteration.saturating_add(1);
        self.save_or_log();
    }

    #[must_use]
    pub const fn loop_exceeded_max(&self) -> bool {
        self.loop_active && self.loop_iteration >= self.loop_max_iterations
    }

    #[must_use]
    pub fn loop_target_reached(&self) -> bool {
        if !self.loop_active {
            return true;
        }
        match self.loop_target.as_str() {
            "kanban:empty" => matches!(self.loop_kanban_runnable, Some(0)),
            "goal" => self.goal_receipt_pass || self.goal_achieved,
            t if t.starts_with("phase:") => {
                let target_phase = t.strip_prefix("phase:").unwrap_or("");
                self.current_phase == target_phase
            }
            _ => false,
        }
    }

    pub fn stop_loop(&mut self) {
        self.loop_active = false;
        self.save_or_log();
    }

    pub fn record_token_spend(&mut self, tokens: i32) {
        self.token_budget_used = self.token_budget_used.saturating_add(tokens.max(0));
        self.save_or_log();
    }

    #[must_use]
    pub const fn budget_exceeded(&self) -> bool {
        self.token_budget_total > 0 && self.token_budget_used >= self.token_budget_total
    }
}
