use crate::state::SessionState;

impl SessionState {
    #[must_use]
    pub const fn has_pending_tests(&self) -> bool {
        !self.test_files_pending.is_empty()
    }

    #[must_use]
    pub const fn should_block_for_tests(&self) -> bool {
        false
    }

    pub fn clear_test_pending(&mut self) {
        self.test_files_pending.clear();
        self.test_nudge_count = 0;
        self.save_or_log();
    }
}
