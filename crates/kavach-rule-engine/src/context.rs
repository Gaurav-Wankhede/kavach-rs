//! Evaluation context: input data for rule evaluation.

use serde::{Deserialize, Serialize};

/// Session phase for token budget awareness.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_enums,
    reason = "SessionPhase variants constructed in-crate via with_phase() builder"
)]
pub enum SessionPhase {
    #[default]
    Early,
    Mid,
    Late,
    Critical,
}

/// Input context for rule evaluation — ABAC attribute carrier.
///
/// Attributes follow NIST SP 800-162 ABAC categories:
/// - Subject: `session_id`, `intent_risk`, `current_phase`
/// - Resource: `file_path`, content
/// - Action: `tool_name`
/// - Environment: `research_done`, `session_phase`, `turn_count`, `loop_active`
///
/// SOURCE: nvlpubs.nist.gov/nistpubs/specialpublications/nist.sp.800-162.pdf
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "EvalContext constructed in-crate via builder pattern in new() and with_* methods"
)]
pub struct EvalContext {
    // Action attributes
    pub tool_name: String,

    // Resource attributes
    pub file_path: Option<String>,
    pub content: Option<String>,

    // Subject attributes (from kavach-db sessions table)
    pub prompt: String,
    pub session_id: Option<String>,
    pub intent_type: String,
    pub intent_risk: String,
    pub current_phase: String,

    // Environment attributes (runtime context)
    pub research_done: bool,
    pub session_phase: SessionPhase,
    pub turn_count: i32,
    pub loop_active: bool,
    pub loop_iteration: i32,
}

impl EvalContext {
    #[must_use]
    pub fn new(tool_name: &str, prompt: &str) -> Self {
        Self {
            tool_name: tool_name.to_owned(),
            prompt: prompt.to_owned(),
            intent_type: kavach_patterns::classify_intent(prompt),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_file(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_owned());
        self
    }

    #[must_use]
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = Some(content.to_owned());
        self
    }

    #[must_use]
    pub const fn with_research(mut self, done: bool) -> Self {
        self.research_done = done;
        self
    }

    #[must_use]
    pub const fn with_phase(mut self, phase: SessionPhase) -> Self {
        self.session_phase = phase;
        self
    }

    /// Populate ABAC subject attributes from session state.
    #[must_use]
    pub fn with_subject(
        mut self,
        session_id: &str,
        intent_risk: &str,
        current_phase: &str,
    ) -> Self {
        self.session_id = Some(session_id.to_owned());
        intent_risk.clone_into(&mut self.intent_risk);
        current_phase.clone_into(&mut self.current_phase);
        self
    }

    /// Populate ABAC environment attributes from runtime context.
    #[must_use]
    pub const fn with_environment(
        mut self,
        turn_count: i32,
        loop_active: bool,
        loop_iteration: i32,
    ) -> Self {
        self.turn_count = turn_count;
        self.loop_active = loop_active;
        self.loop_iteration = loop_iteration;
        self
    }

    /// True when the current phase is high-risk (medium or high `intent_risk`).
    /// Used by policy rules to gate destructive actions.
    #[must_use]
    pub fn is_high_risk(&self) -> bool {
        matches!(self.intent_risk.as_str(), "medium" | "high" | "critical")
    }

    /// True when we're in an autonomous harness loop.
    #[must_use]
    pub const fn in_loop(&self) -> bool {
        self.loop_active
    }

    #[must_use]
    pub fn is_write_tool(&self) -> bool {
        matches!(self.tool_name.as_str(), "Write" | "Edit" | "NotebookEdit")
    }

    pub fn is_code_target(&self) -> bool {
        self.file_path
            .as_deref()
            .is_some_and(kavach_patterns::is_code_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let ctx = EvalContext::new("Write", "fix the bug");
        assert_eq!(ctx.tool_name, "Write");
        assert!(!ctx.research_done);
    }

    #[test]
    fn test_builder_chain() {
        let ctx = EvalContext::new("Edit", "add feature")
            .with_file("src/main.rs")
            .with_research(true)
            .with_phase(SessionPhase::Mid);
        assert_eq!(ctx.file_path.as_deref(), Some("src/main.rs"));
        assert!(ctx.research_done);
        assert_eq!(ctx.session_phase, SessionPhase::Mid);
    }

    #[test]
    fn test_with_subject_populates_abac_attributes() {
        let ctx = EvalContext::new("Write", "implement feature").with_subject(
            "sess_abc",
            "high",
            "IMPLEMENT",
        );
        assert_eq!(ctx.session_id.as_deref(), Some("sess_abc"));
        assert_eq!(ctx.intent_risk, "high");
        assert_eq!(ctx.current_phase, "IMPLEMENT");
    }

    #[test]
    fn test_with_environment_populates_runtime_attributes() {
        let ctx = EvalContext::new("Bash", "cargo test").with_environment(42, true, 3);
        assert_eq!(ctx.turn_count, 42);
        assert!(ctx.loop_active);
        assert_eq!(ctx.loop_iteration, 3);
        assert!(ctx.in_loop());
    }

    #[test]
    fn test_is_high_risk_classifies_correctly() {
        let ctx = EvalContext::new("Write", "x").with_subject("s", "high", "IMPLEMENT");
        assert!(ctx.is_high_risk());

        let ctx_low = EvalContext::new("Write", "x").with_subject("s", "low", "PLAN");
        assert!(!ctx_low.is_high_risk());

        let ctx_critical = EvalContext::new("Write", "x").with_subject("s", "critical", "HARDEN");
        assert!(ctx_critical.is_high_risk());
    }

    #[test]
    fn test_default_context_is_safe() {
        let ctx = EvalContext::default();
        assert!(!ctx.in_loop());
        assert!(!ctx.is_high_risk());
        assert_eq!(ctx.session_phase, SessionPhase::Early);
    }
}
