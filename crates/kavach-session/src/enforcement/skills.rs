use crate::state::SessionState;

impl SessionState {
    pub fn set_required_skills(&mut self, skills: Vec<String>) {
        self.required_skills = skills;
        self.invoked_skills.clear();
        self.save_or_log();
    }

    pub fn record_skill_invoked(&mut self, skill_name: &str) {
        const ARCH_SKILL_NAME: &str = "arch";
        let name = skill_name.to_owned();
        let is_arch = skill_name == ARCH_SKILL_NAME;
        if let Err(e) = self.atomic_update(|s| {
            if !s.invoked_skills.iter().any(|x| x == &name) {
                s.invoked_skills.push(name.clone());
            }
            if is_arch {
                s.arch_skill_invoked = true;
            }
        }) {
            tracing::warn!(error = %e, "record_skill_invoked atomic_update failed; falling back to non-atomic");
            if !self.invoked_skills.iter().any(|s| s == skill_name) {
                self.invoked_skills.push(skill_name.into());
            }
            if is_arch {
                self.arch_skill_invoked = true;
            }
            self.save_or_log();
        }
    }

    #[must_use]
    pub fn missing_skills(&self) -> Vec<&str> {
        self.required_skills
            .iter()
            .filter(|req| !self.invoked_skills.iter().any(|inv| inv == req.as_str()))
            .map(String::as_str)
            .collect()
    }
}
