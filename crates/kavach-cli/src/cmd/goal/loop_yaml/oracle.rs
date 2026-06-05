// Oracle proof signal + max-attempts policy for the goal-loop data model.
//
// SOURCE: github.com/tolibear/goalbuddy (oracle concept) · decision.goal-oracle-workflow.
use serde::{Deserialize, Serialize};

/// The observable proof signal that decides whether a goal is actually done.
/// `TestExitCode` is the MVP (a command whose exit code is proof).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum Oracle {
    /// Run `check`; exit code 0 is a pass. Optional `expect_contains` is a
    /// second assertion against captured stdout.
    TestExitCode {
        /// The shell command whose exit code is the proof signal.
        check: String,
        /// Optional stdout substring that must also be present.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expect_contains: Option<String>,
    },
    /// A predicate expression (e.g. "file exists", "benchmark > N").
    Predicate {
        /// The predicate string the loop evaluates.
        check: String,
    },
    /// A human must approve. Loop pauses, emits an awaiting-approval receipt.
    Human {
        /// The approval prompt shown to the human.
        prompt: String,
    },
}

impl Oracle {
    /// The shell/predicate string the loop runs each attempt. For `Human`,
    /// returns the approval prompt (the loop pauses rather than executing it).
    pub(crate) fn check_str(&self) -> &str {
        match self {
            Self::TestExitCode { check, .. } | Self::Predicate { check } => check,
            Self::Human { prompt } => prompt,
        }
    }
}

/// What to do when `max_attempts` is exhausted without a passing oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OnMaxAttempts {
    /// Alert the user and stop — never silently fail or fake success.
    Escalate,
    /// Conclude the goal is unmet.
    Fail,
    /// Pause and wait for explicit human approval.
    WaitManualApproval,
}
