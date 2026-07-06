use std::fmt;
use std::io;

/// Structured error context for gate failures.
/// Enables chain runner to make retry/skip/escalate decisions.
#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed via struct literal at gate call sites; non_exhaustive => E0639"
)]
pub struct GateErrorContext {
    pub gate_name: String,
    pub error_category: ErrorCategory,
    pub attempted_action: String,
    pub is_retryable: bool,
}

/// Failure categories for structured error propagation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively by the chain runner retry/skip/escalate logic; non_exhaustive => E0004"
)]
pub enum ErrorCategory {
    /// Timeout, rate limit, connection refused — worth retrying.
    Transient,
    /// Bad input, format error — fix input then retry.
    Validation,
    /// Policy violation, blocked by gate — not retryable.
    Policy,
    /// Access denied — escalate to user.
    Permission,
}

impl fmt::Display for GateErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "gate={} category={:?} action={} retryable={}",
            self.gate_name, self.error_category, self.attempted_action, self.is_retryable,
        )
    }
}

// SOURCE: https://docs.rs/miette/7 — Diagnostic adds severity + help + url to errors.
// miette renders these with file/line context when reported via miette::Result.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively at engine error-handling boundaries; non_exhaustive => E0004"
)]
pub enum EngineError {
    #[error("unknown gate: {0}")]
    #[diagnostic(
        code(kavach::engine::unknown_gate),
        help("Check the gate name against the kavach-engine wiring map in CLAUDE.md")
    )]
    UnknownGate(String),

    #[error("session error: {0}")]
    #[diagnostic(
        code(kavach::engine::session),
        help("Inspect ~/.claude/sessions/<id>.json for state corruption")
    )]
    Session(String),

    #[error("io error: {0}")]
    #[diagnostic(code(kavach::engine::io))]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    #[diagnostic(
        code(kavach::engine::json),
        help("Verify the hook payload matches kavach_types::HookInput schema")
    )]
    Json(#[from] serde_json::Error),

    #[error("gate verdict: {0}")]
    #[diagnostic(
        severity(Error),
        code(kavach::engine::blocked),
        help("Re-read the gate's policy rule in CLAUDE.md and adjust the action")
    )]
    Blocked(String),

    #[error("gate error: {0}")]
    #[diagnostic(code(kavach::engine::gate_error))]
    GateError(GateErrorContext),
}
