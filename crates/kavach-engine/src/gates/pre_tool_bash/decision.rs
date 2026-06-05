//! Uniform gate verdict so the dispatcher maps to a single hook-exit call.

/// A Bash pre-tool gate verdict. The dispatcher translates each to the matching
/// `kavach_hook::exit_pre_tool_*` call exactly once.
pub(super) enum Decision {
    /// Hard refuse (P0).
    Deny(String),
    /// Confirm prompt (P1).
    Ask(String),
    /// Allow, optionally with advisory context (None = silent allow).
    Allow(Option<String>),
}

impl Decision {
    /// Fire the matching hook exit for this verdict.
    pub(super) fn emit(self) {
        match self {
            Self::Deny(reason) => drop(kavach_hook::exit_pre_tool_deny(&reason)),
            Self::Ask(reason) => drop(kavach_hook::exit_pre_tool_ask(&reason)),
            Self::Allow(ctx) => drop(kavach_hook::exit_pre_tool_allow(ctx.as_deref())),
        }
    }
}
