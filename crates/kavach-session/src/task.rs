use crate::state::SessionState;

impl SessionState {
    pub fn set_current_task(&mut self, task: &str) {
        if self.current_task != task && !task.is_empty() {
            self.current_task = task.into();
            self.research_done = false;
            self.task_status = "in_progress".into();
            self.save_or_log();
        }
    }

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

    pub fn clear_task(&mut self) {
        self.current_task.clear();
        self.task_status.clear();
        self.files_modified.clear();
        self.save_or_log();
    }

    #[must_use]
    pub const fn has_task(&self) -> bool {
        !self.current_task.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_add_file_modified() {
        let mut s = SessionState::default();
        assert!(s.add_file_modified("a.rs"));
        assert!(!s.add_file_modified("a.rs"));
        assert!(s.add_file_modified("b.rs"));
        assert_eq!(s.files_modified.len(), 2);
    }

    #[test]
    fn test_has_task() {
        let mut s = SessionState::default();
        assert!(!s.has_task());
        s.current_task = "test".into();
        assert!(s.has_task());
    }
}
