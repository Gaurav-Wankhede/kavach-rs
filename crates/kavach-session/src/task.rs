// SOURCE: ~/.claude/CLAUDE.md Code form § nano-files: tests in separate mapped files
use crate::state::SessionState;

impl SessionState {
    pub fn set_task(&mut self, task: &str, status: &str) {
        self.current_task = task.into();
        self.task_status = status.into();
        self.save_or_log();
    }

    pub fn add_file_modified(&mut self, file_path: &str) -> bool {
        if self.files_modified.iter().any(|f| f == file_path) {
            return false;
        }
        self.files_modified.push(file_path.into());
        self.save_or_log();
        true
    }

    #[must_use]
    pub const fn has_task(&self) -> bool {
        !self.current_task.is_empty()
    }
}
