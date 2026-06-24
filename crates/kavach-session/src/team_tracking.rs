use crate::state::SessionState;

impl SessionState {
    /// Recompute `active_teammates` from `team_members` (single source of truth).
    /// The scalar is what the stop gate reads and what serializes; deriving it
    /// after every mutation makes start/stop drift — a no-match stop that
    /// decrements nothing, or a recycled name — structurally impossible.
    fn recount_active_teammates(&mut self) {
        self.active_teammates = i32::try_from(self.team_members.len()).unwrap_or(i32::MAX);
    }

    pub fn track_teammate_stop(&mut self, name: &str) {
        // retain may remove 0 entries (name absent, recycled, double-stop). The
        // old always-decrement-if-positive guard drifted the count below real
        // membership in exactly that case; recounting from the Vec cannot.
        self.team_members
            .retain(|m| !m.starts_with(&format!("{name}:")));
        self.recount_active_teammates();
        self.save_or_log();
    }

    #[must_use]
    pub const fn is_in_team(&self) -> bool {
        !self.team_name.is_empty()
    }

    pub fn set_team(&mut self, team_name: &str) {
        self.team_name = team_name.into();
        self.save_or_log();
    }

    #[must_use]
    pub fn team_summary(&self) -> String {
        format!(
            "team={} members={} active={}",
            self.team_name,
            self.team_members.len(),
            self.active_teammates
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_set_team() {
        let mut s = SessionState::default();
        assert!(!s.is_in_team());
        s.set_team("my-team");
        assert!(s.is_in_team());
        assert_eq!(s.team_name, "my-team");
    }

}
