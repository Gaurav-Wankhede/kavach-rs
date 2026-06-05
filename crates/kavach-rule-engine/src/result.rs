//! Rule evaluation output types.

use serde::{Deserialize, Serialize};

/// Action the engine recommends after rule evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RuleAction {
    Allow,
    Block,
    Warn,
    Modify,
}

/// Result of evaluating a single rule against context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at RPC handler boundary via RuleResult::allow/block/warn methods"
)]
pub struct RuleResult {
    pub action: RuleAction,
    pub rule_name: String,
    pub reason: String,
    pub skill_name: Option<String>,
    pub severity: u8,
}

impl RuleResult {
    #[must_use]
    pub fn allow(rule_name: &str, reason: &str) -> Self {
        Self {
            action: RuleAction::Allow,
            rule_name: rule_name.to_owned(),
            reason: reason.to_owned(),
            skill_name: None,
            severity: 0,
        }
    }

    #[must_use]
    pub fn block(rule_name: &str, reason: &str, severity: u8) -> Self {
        Self {
            action: RuleAction::Block,
            rule_name: rule_name.to_owned(),
            reason: reason.to_owned(),
            skill_name: None,
            severity,
        }
    }

    #[must_use]
    pub fn warn(rule_name: &str, reason: &str, severity: u8) -> Self {
        Self {
            action: RuleAction::Warn,
            rule_name: rule_name.to_owned(),
            reason: reason.to_owned(),
            skill_name: None,
            severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_result() {
        let r = RuleResult::allow("test", "passed");
        assert_eq!(r.action, RuleAction::Allow);
        assert_eq!(r.severity, 0);
    }

    #[test]
    fn test_block_result() {
        let r = RuleResult::block("stub_check", "found todo!()", 9);
        assert_eq!(r.action, RuleAction::Block);
        assert_eq!(r.severity, 9);
    }

    #[test]
    fn test_warn_result() {
        let r = RuleResult::warn("research", "no search done", 5);
        assert_eq!(r.action, RuleAction::Warn);
        assert_eq!(r.reason, "no search done");
    }
}
