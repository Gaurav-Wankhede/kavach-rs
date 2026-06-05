use crate::state::SessionState;

impl SessionState {
    pub fn store_intent(&mut self, intent_type: &str, domain: &str, skills: Vec<String>) {
        self.intent_type = intent_type.into();
        self.intent_domain = domain.into();
        self.intent_skills = skills;
        self.save_or_log();
    }

    /// Authoritatively set the per-turn intent classification.
    ///
    /// `intent_type` is a pure function of the CURRENT prompt, recomputed
    /// every turn — but `parse.rs` load-guards it (`if
    /// self.intent_type.is_empty()`), so a stale persisted value would
    /// otherwise shadow the fresh classification for the whole session
    /// (rca.intent-classifier-stuck-implement — cache-without-invalidation,
    /// `LiteLLM` #25553 class). The intent gate calls this every turn AFTER
    /// `analyze_intent`, so the freshly-derived value always wins.
    /// Single-responsibility vs `store_intent`: it does NOT touch
    /// `intent_domain`/`intent_skills` (the gate sets those independently
    /// via `set_required_skills` — avoiding a clobber). Mirrors the
    /// existing per-prompt write `set_intent_risk`.
    pub fn set_intent_type(&mut self, intent_type: &str) {
        self.intent_type = intent_type.into();
        self.save_or_log();
    }

    pub fn mark_spec_injected(&mut self, name: &str) {
        if !self.specs_injected.iter().any(|n| n == name) {
            self.specs_injected.push(name.into());
            self.save_or_log();
        }
    }

    #[must_use]
    pub fn was_spec_injected(&self, name: &str) -> bool {
        self.specs_injected.iter().any(|n| n == name)
    }

    /// Detect if prompt is an explicit user confirmation to create a new crate.
    ///
    /// Matches explicit multi-word phrases ("yes create", "proceed with crate")
    /// and bare affirmatives ("yes", "y", "ok", "proceed") — consistent with
    /// dialoguer/inquire conventions where a single "y"/"yes" is a valid reply.
    #[must_use]
    pub fn detect_new_crate_confirmation(prompt: &str) -> bool {
        let lower = prompt.trim().to_lowercase();
        let phrases = [
            "create new crate",
            "create the crate",
            "yes create",
            "proceed with crate",
            "go ahead create",
            "yes proceed",
            "create it",
            "yes, create",
            "confirmed",
            "create new service",
            "create the service",
            "create new package",
        ];
        if phrases.iter().any(|p| lower.contains(p)) {
            return true;
        }
        let bare = [
            "yes", "y", "yep", "yeah", "yup", "ok", "okay", "proceed", "go", "do it",
        ];
        bare.iter().any(|a| lower == *a)
    }

    pub fn confirm_new_crate(&mut self) {
        self.new_crate_confirmed = true;
        self.save_or_log();
    }

    pub fn clear_new_crate_confirmed(&mut self) {
        self.new_crate_confirmed = false;
        self.save_or_log();
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_store_intent() {
        let mut s = SessionState::default();
        s.store_intent("implement", "backend", vec!["/rust".into()]);
        assert_eq!(s.intent_type, "implement");
        assert_eq!(s.intent_domain, "backend");
        assert_eq!(s.intent_skills, vec!["/rust"]);
    }

    #[test]
    fn set_intent_type_overwrites_stale_value() {
        // REGRESSION (rca.intent-classifier-stuck-implement): turn-1 persisted
        // intent_type="implement"; parse.rs:51 load-guard refused to clobber
        // it, so every later turn's fresh classification was shadowed →
        // RCA/skill gates misfired all session. set_intent_type MUST
        // authoritatively overwrite the stale value (cache-without-
        // invalidation fix — the per-turn-derived value always wins).
        let mut s = SessionState::default();
        s.intent_type = "implement".into(); // simulate stale turn-1 / parse.rs-loaded value
        s.set_intent_type("research"); // turn-N fresh classification
        assert_eq!(
            s.intent_type, "research",
            "fresh per-turn classification MUST overwrite the stale latched value"
        );
    }

    #[test]
    fn set_intent_type_does_not_clobber_domain_or_skills() {
        // Single-responsibility guarantee: the gate sets intent_domain /
        // intent_skills independently (via set_required_skills). If
        // set_intent_type also reset them it would erase the skill list →
        // skill-gate deadlock. It must touch ONLY intent_type.
        let mut s = SessionState::default();
        s.store_intent("debug", "backend", vec!["/bug-bounty".into()]);
        s.set_intent_type("research");
        assert_eq!(s.intent_type, "research");
        assert_eq!(
            s.intent_domain, "backend",
            "set_intent_type must NOT clobber intent_domain"
        );
        assert_eq!(
            s.intent_skills,
            vec!["/bug-bounty"],
            "set_intent_type must NOT clobber intent_skills"
        );
    }

    #[test]
    fn test_detect_confirmation_phrases() {
        assert!(SessionState::detect_new_crate_confirmation("yes create it"));
        assert!(SessionState::detect_new_crate_confirmation(
            "proceed with crate"
        ));
    }

    #[test]
    fn test_detect_confirmation_bare() {
        assert!(SessionState::detect_new_crate_confirmation("yes"));
        assert!(SessionState::detect_new_crate_confirmation("y"));
        assert!(SessionState::detect_new_crate_confirmation("  OK  "));
        assert!(SessionState::detect_new_crate_confirmation("proceed"));
    }

    #[test]
    fn test_detect_confirmation_rejects_unrelated() {
        assert!(!SessionState::detect_new_crate_confirmation("no"));
        assert!(!SessionState::detect_new_crate_confirmation("maybe"));
        assert!(!SessionState::detect_new_crate_confirmation(""));
        assert!(!SessionState::detect_new_crate_confirmation(
            "yes but not really"
        ));
    }

    #[test]
    fn test_spec_injected() {
        let mut s = SessionState::default();
        assert!(!s.was_spec_injected("api"));
        s.mark_spec_injected("api");
        assert!(s.was_spec_injected("api"));
        s.mark_spec_injected("api");
        assert_eq!(s.specs_injected.len(), 1);
    }
}
