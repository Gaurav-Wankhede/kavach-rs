use crate::state::SessionState;

impl SessionState {
    /// Check if a module has already been injected this session.
    #[must_use]
    pub fn has_module(&self, name: &str) -> bool {
        self.modules_injected.iter().any(|m| m == name)
    }

    /// Mark a module as injected for this session.
    pub fn mark_module(&mut self, name: &str) {
        if !self.has_module(name) {
            self.modules_injected.push(name.into());
            _ = self.save();
        }
    }

    /// Inject modules: full content for new ones, one-liner for already-loaded.
    /// Returns the combined string to append to hook context.
    pub fn inject_modules_once(&mut self, names: &[&str]) -> String {
        let mut new_names: Vec<&str> = Vec::new();
        let mut already: Vec<&str> = Vec::new();

        for &name in names {
            if self.has_module(name) {
                already.push(name);
            } else {
                new_names.push(name);
                self.mark_module(name);
            }
        }

        let mut out = String::new();

        if !new_names.is_empty() {
            let content = kavach_config::load_modules(&new_names);
            if !content.is_empty() {
                out.push_str("\n[MODULE:LAZY_LOADED]\n");
                out.push_str(&content);
            }
        }

        // Already-loaded modules: emit nothing. Session state tracks them.
        // Emitting a reminder on every tool call is pure token waste.
        let _ = already;

        _ = self.save();
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_has_module_empty() {
        let s = SessionState::default();
        assert!(!s.has_module("agi-flow"));
    }

    #[test]
    fn test_mark_module() {
        let mut s = SessionState::default();
        s.mark_module("agi-flow");
        assert!(s.has_module("agi-flow"));
        assert!(!s.has_module("memory"));
    }

    #[test]
    fn test_mark_module_idempotent() {
        let mut s = SessionState::default();
        s.mark_module("agi-flow");
        s.mark_module("agi-flow");
        assert_eq!(s.modules_injected.len(), 1);
    }
}
