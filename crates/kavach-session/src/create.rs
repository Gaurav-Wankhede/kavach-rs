#[cfg(test)]
mod tests {
    use crate::enforcement::generate_session_id;
    use crate::paths::today;
    use crate::state::SessionState;

    #[test]
    fn test_generate_session_id() {
        let id = generate_session_id("/tmp/test");
        assert!(id.starts_with("sess_"));
        assert_eq!(id.len(), 5 + 32);
    }

    #[test]
    fn test_new_session() {
        let s = SessionState::new("/tmp/test");
        assert!(s.id.starts_with("sess_"));
        assert_eq!(s.today, today());
        assert_eq!(s.token_budget_total, 180_000);
        assert_eq!(s.context_phase, "early");
        assert_eq!(s.subagent_max_output, 8_000);
    }
}
