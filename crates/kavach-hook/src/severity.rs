// GateSeverity: 3-tier classification for kavach gates.
// SOURCE: roadmap.unit.gate-severity-router — fixes anti-pattern where gates
// used binary block (exit 2) for advisory workload, training agents to halt
// and ask the user. Maps to Claude Code 2.x hookSpecificOutput.permissionDecision.
// SOURCE: code.claude.com/docs/en/hooks (permissionDecision: allow|ask|deny)

/// Gate decision severity. Maps 1:1 to Claude Code 2.x permission decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively in permission_decision + cross-crate gate routing; non_exhaustive => E0004"
)]
pub enum GateSeverity {
    /// Irreversible-AND-FP-bounded-below-1%. Reserved for: destructive-cli,
    /// secret-detector, banned-crypto, rls-bypass, `unsafe_code`. ≤ 5 sites total.
    P0Block,
    /// Reversible-but-risky. User prompt; agent continues planning while waiting.
    P1Ask,
    /// Methodology nudge / quality hint. Edit proceeds; reason injected into
    /// next-turn context via additionalContext. The default tier for new gates.
    P2Advise,
}

impl GateSeverity {
    /// Canonical Claude Code 2.x permissionDecision wire value.
    #[must_use]
    pub const fn permission_decision(self) -> &'static str {
        match self {
            Self::P0Block => "deny",
            Self::P1Ask => "ask",
            Self::P2Advise => "allow",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p0_maps_to_deny() {
        assert_eq!(GateSeverity::P0Block.permission_decision(), "deny");
    }
    #[test]
    fn p1_maps_to_ask() {
        assert_eq!(GateSeverity::P1Ask.permission_decision(), "ask");
    }
    #[test]
    fn p2_maps_to_allow() {
        assert_eq!(GateSeverity::P2Advise.permission_decision(), "allow");
    }
}
