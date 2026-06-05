use crate::state::SessionState;

impl SessionState {
    /// Compact session state for context injection.
    #[must_use]
    pub fn to_compact(&self) -> String {
        let research = if self.research_done {
            "DONE"
        } else {
            "PENDING"
        };
        let memory = if self.memory_queried {
            "DONE"
        } else {
            "PENDING"
        };
        format!(
            "[SESSION]\nid: {}\ntoday: {}\nproject: {}\n\
             research: {}\nmemory: {}\ncutoff: {}\n",
            self.id, self.today, self.project, research, memory, self.training_cutoff
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_to_compact() {
        let s = SessionState::new("/tmp/test");
        let toon = s.to_compact();
        assert!(toon.contains("[SESSION]"));
        assert!(toon.contains("research: PENDING"));
        assert!(toon.contains("memory: PENDING"));
    }
}
