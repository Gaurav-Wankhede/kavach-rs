use crate::state::SessionState;

#[derive(Debug, Clone)]
#[expect(clippy::exhaustive_structs, reason = "constructed at handler")]
pub struct SessionIdentity {
    pub id: String,
    pub today: String,
    pub project: String,
    pub work_dir: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
#[expect(clippy::exhaustive_structs, reason = "constructed at handler")]
pub struct SessionFlags {
    pub id: String,
    pub today: String,
    pub project: String,
    pub work_dir: String,
    pub session_id: String,
    pub research_done: bool,
    pub memory_queried: bool,
}

#[derive(Debug, Clone)]
#[expect(clippy::exhaustive_structs, reason = "constructed at handler")]
pub struct SessionTracking {
    pub id: String,
    pub today: String,
    pub project: String,
    pub work_dir: String,
    pub session_id: String,
    pub research_done: bool,
    pub memory_queried: bool,
    pub turn_count: i32,
    pub post_compact: bool,
    pub current_task: String,
    pub tasks_created: i32,
    pub tasks_completed: i32,
}

impl SessionState {
    #[must_use]
    pub fn identity(&self) -> SessionIdentity {
        SessionIdentity {
            id: self.id.clone(),
            today: self.today.clone(),
            project: self.project.clone(),
            work_dir: self.work_dir.clone(),
            session_id: self.session_id.clone(),
        }
    }

    #[must_use]
    pub fn flags(&self) -> SessionFlags {
        SessionFlags {
            id: self.id.clone(),
            today: self.today.clone(),
            project: self.project.clone(),
            work_dir: self.work_dir.clone(),
            session_id: self.session_id.clone(),
            research_done: self.research_done,
            memory_queried: self.memory_queried,
        }
    }

    #[must_use]
    pub fn tracking(&self) -> SessionTracking {
        SessionTracking {
            id: self.id.clone(),
            today: self.today.clone(),
            project: self.project.clone(),
            work_dir: self.work_dir.clone(),
            session_id: self.session_id.clone(),
            research_done: self.research_done,
            memory_queried: self.memory_queried,
            turn_count: self.turn_count,
            post_compact: self.post_compact,
            current_task: self.current_task.clone(),
            tasks_created: self.tasks_created,
            tasks_completed: self.tasks_completed,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_identity_subset() {
        let s = SessionState::new("/tmp/test");
        let id = s.identity();
        assert_eq!(id.id, s.id);
        assert_eq!(id.today, s.today);
    }
}
