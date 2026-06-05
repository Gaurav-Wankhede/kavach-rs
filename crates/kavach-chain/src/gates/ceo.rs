use crate::chain_state::ChainState;
use crate::types::{CEODecision, IntentAnalysis, VerificationResult};
use std::collections::HashMap;

pub(crate) fn run_gate(state: &mut ChainState, tool_name: &str, agent_type: &str) {
    let ceo = ceo_validate(state.intent.as_ref(), tool_name, agent_type);

    let mut result = VerificationResult {
        gate: "CEO".into(),
        status: "pass".into(),
        reason: "Delegation strategy validated".into(),
        context: HashMap::new(),
        timestamp: String::new(),
        next_action: String::new(),
    };

    if !ceo.approved {
        result.status = "block".into();
        result.reason = ceo.blockers.join("; ");
        result.next_action = "Provide required parameters or clarify task".into();
    } else if !ceo.warnings.is_empty() {
        result.status = "warn".into();
        result.reason = ceo.warnings.join("; ");
    }

    if !ceo.delegation_plan.is_empty() {
        result
            .context
            .insert("plan".into(), ceo.delegation_plan.clone());
    }

    state.ceo = Some(ceo);
    state.add_result(result);
}

#[must_use]
pub fn ceo_validate(
    intent: Option<&IntentAnalysis>,
    tool_name: &str,
    agent_type: &str,
) -> CEODecision {
    let mut d = CEODecision {
        approved: true,
        delegation_plan: String::new(),
        assigned_agents: Vec::new(),
        task_breakdown: Vec::new(),
        blockers: Vec::new(),
        warnings: Vec::new(),
    };

    if tool_name == "Task" && agent_type.is_empty() {
        d.approved = false;
        d.blockers.push("Task requires subagent_type".into());
        return d;
    }

    if let Some(intent) = intent {
        if !intent.required_agents.is_empty()
            && !agent_type.is_empty()
            && !intent.required_agents.iter().any(|a| a == agent_type)
        {
            d.warnings.push(format!(
                "Agent '{agent_type}' may not be optimal for intent '{}'",
                intent.intent_type
            ));
        }
        if intent.risk_level == "critical" {
            d.warnings
                .push("CRITICAL risk level - verify user intent before proceeding".into());
        }
        if intent.complexity == "complex" {
            d.delegation_plan = "Complex task - recommend task breakdown".into();
            d.task_breakdown = vec![
                "1. Research current patterns".into(),
                "2. Create implementation plan".into(),
                "3. Implement with verification".into(),
                "4. Test and validate".into(),
            ];
        }
    }

    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceo_validate_task_no_agent() {
        let d = ceo_validate(None, "Task", "");
        assert!(!d.approved);
        assert!(!d.blockers.is_empty());
    }

    #[test]
    fn test_ceo_validate_pass() {
        let d = ceo_validate(None, "Read", "");
        assert!(d.approved);
    }
}
