use crate::state::SessionState;

impl SessionState {
    /// Recompute `active_teammates` from `team_members` (single source of truth).
    /// The scalar is what the stop gate reads and what serializes; deriving it
    /// after every mutation makes start/stop drift — a no-match stop that
    /// decrements nothing, or a recycled name — structurally impossible.
    fn recount_active_teammates(&mut self) {
        self.active_teammates = i32::try_from(self.team_members.len()).unwrap_or(i32::MAX);
    }

    pub fn track_teammate_start(&mut self, name: &str, agent_type: &str) {
        let entry = format!("{name}:{agent_type}");
        if !self.team_members.contains(&entry) {
            self.team_members.push(entry);
        }
        self.recount_active_teammates();
        self.save_or_log();
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
    fn test_teammate_tracking() {
        let mut s = SessionState::default();
        s.track_teammate_start("researcher", "Explore");
        assert_eq!(s.active_teammates, 1);
        assert!(s.team_members.contains(&"researcher:Explore".to_owned()));
        s.track_teammate_stop("researcher");
        assert_eq!(s.active_teammates, 0);
        assert!(s.team_members.is_empty());
    }

    #[test]
    fn test_teammate_count_stays_derived_from_members() {
        // A stop with no matching member must not drift the count below the real
        // membership — the old always-decrement guard did exactly that.
        let mut s = SessionState::default();
        s.track_teammate_start("a", "Explore");
        s.track_teammate_start("b", "Code");
        assert_eq!(s.active_teammates, 2);
        s.track_teammate_stop("ghost"); // no match — removes nothing
        assert_eq!(s.active_teammates, 2);
        assert_eq!(s.team_members.len(), 2);
        s.track_teammate_stop("a"); // double-stop "a" below also a no-op
        s.track_teammate_stop("a");
        assert_eq!(s.active_teammates, 1);
        assert_eq!(
            s.active_teammates,
            i32::try_from(s.team_members.len()).unwrap_or(i32::MAX)
        );
    }

    #[test]
    fn test_duplicate_start_does_not_inflate_count() {
        let mut s = SessionState::default();
        s.track_teammate_start("a", "Explore");
        s.track_teammate_start("a", "Explore"); // dedup'd by team_members
        assert_eq!(s.active_teammates, 1);
        assert_eq!(s.team_members.len(), 1);
    }

    #[test]
    fn test_set_team() {
        let mut s = SessionState::default();
        assert!(!s.is_in_team());
        s.set_team("my-team");
        assert!(s.is_in_team());
        assert_eq!(s.team_name, "my-team");
    }

    #[test]
    fn test_team_summary() {
        let mut s = SessionState::default();
        s.set_team("alpha");
        s.track_teammate_start("eng", "Code");
        let summary = s.team_summary();
        assert!(summary.contains("team=alpha"));
        assert!(summary.contains("members=1"));
        assert!(summary.contains("active=1"));
    }
}
