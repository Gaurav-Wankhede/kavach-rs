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
