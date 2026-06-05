//! SP/1.0 protocol compliance checks.

use crate::context::{EvalContext, SessionPhase};
use crate::result::{RuleAction, RuleResult};
use kavach_patterns::detect_antiprod;

/// Run all SP/1.0 compliance checks against the evaluation context.
#[must_use]
pub fn check_sp_compliance(ctx: &EvalContext) -> Vec<RuleResult> {
    let mut results = Vec::new();
    if let Some(r) = check_antiprod_violations(ctx) {
        results.extend(r);
    }
    if let Some(r) = check_tool_usage_pattern(ctx) {
        results.push(r);
    }
    if let Some(r) = check_session_phase_limits(ctx) {
        results.push(r);
    }
    results
}

/// Check for anti-production patterns in write content.
fn check_antiprod_violations(ctx: &EvalContext) -> Option<Vec<RuleResult>> {
    let content = ctx.content.as_deref()?;
    let file_path = ctx.file_path.as_deref()?;
    if content.is_empty() {
        return None;
    }
    let violations = detect_antiprod(file_path, content);
    if violations.is_empty() {
        return None;
    }
    let results = violations
        .iter()
        .map(|v| RuleResult {
            action: RuleAction::Warn,
            rule_name: format!("antiprod:{}", v.code),
            reason: format!("{}: {}", v.match_text, v.message),
            skill_name: None,
            severity: match v.code {
                "MOCK_DATA" => 9,
                "PROD_LEAK" => 7,
                "ERROR_BLIND" => 6,
                _ => 4,
            },
        })
        .collect();
    Some(results)
}

/// Check that tool usage follows SP/1.0 patterns.
fn check_tool_usage_pattern(ctx: &EvalContext) -> Option<RuleResult> {
    if ctx.is_write_tool() && !ctx.research_done && ctx.is_code_target() {
        return Some(RuleResult::warn(
            "sp_tool_order",
            "SP/1.0 requires research before code writes",
            7,
        ));
    }
    None
}

/// Enforce session phase limits on tool usage.
fn check_session_phase_limits(ctx: &EvalContext) -> Option<RuleResult> {
    match ctx.session_phase {
        SessionPhase::Critical => Some(RuleResult::block(
            "session_phase",
            "Critical phase: block new writes to conserve context",
            9,
        )),
        SessionPhase::Late if ctx.is_write_tool() => Some(RuleResult::warn(
            "session_phase",
            "Late phase: minimize writes to conserve context",
            5,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_violations_clean() {
        let ctx = EvalContext::new("Write", "fix bug")
            .with_file("src/main.rs")
            .with_content("fn main() -> Result<(), Box<dyn std::error::Error>> { Ok(()) }")
            .with_research(true);
        let r = check_sp_compliance(&ctx);
        assert!(r.is_empty() || r.iter().all(|x| x.action != RuleAction::Block));
    }

    #[test]
    fn test_critical_phase_blocks() {
        let ctx = EvalContext::new("Write", "add code").with_phase(SessionPhase::Critical);
        let r = check_sp_compliance(&ctx);
        assert!(r.iter().any(|x| x.action == RuleAction::Block));
    }

    #[test]
    fn test_late_phase_warns() {
        let ctx = EvalContext::new("Write", "add code").with_phase(SessionPhase::Late);
        let r = check_sp_compliance(&ctx);
        assert!(r.iter().any(|x| x.rule_name == "session_phase"));
    }
}
