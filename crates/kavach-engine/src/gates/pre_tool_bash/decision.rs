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
    /// The bandit action this verdict represents (Layer-A logging).
    pub(super) const fn action(&self) -> kavach_patterns::bandit_log::GateAction {
        use kavach_patterns::bandit_log::GateAction;
        match self {
            Self::Deny(_) => GateAction::Block,
            Self::Ask(_) => GateAction::Ask,
            Self::Allow(_) => GateAction::Allow,
        }
    }

    /// Fire the matching hook exit for this verdict.
    pub(super) fn emit(self, session: &mut kavach_session::SessionState) {
        match self {
            Self::Deny(reason) => drop(kavach_hook::exit_pre_tool_deny(&reason)),
            Self::Ask(reason) => drop(kavach_hook::exit_pre_tool_ask(&reason)),
            Self::Allow(ctx) => {
                crate::gates::turn_relay::exit_pre_tool_allow_relay(session, ctx.as_deref());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Decision;
    use kavach_patterns::bandit_log::GateAction;

    #[test]
    fn action_maps_each_verdict_to_its_bandit_action() {
        // Security invariant: a Deny must log as Block, never as Allow — the
        // bandit reward signal would otherwise mis-learn a hard refuse as a pass.
        assert_eq!(Decision::Deny("x".into()).action(), GateAction::Block);
        assert_eq!(Decision::Ask("x".into()).action(), GateAction::Ask);
        assert_eq!(Decision::Allow(None).action(), GateAction::Allow);
        assert_eq!(
            Decision::Allow(Some("ctx".into())).action(),
            GateAction::Allow
        );
    }
}
