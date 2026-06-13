//! Build the `[INTENT]` key-value block: classification, temporal context,
//! and skill/agent routing hints.

/// Risk labels below this confidence are guesses — printing `risk: critical`
/// off a coin-flip classification was gate noise that skewed every turn.
/// Below the floor the line is OMITTED (the session still stores the raw
/// risk for enforcement gates; only the printed advisory is gated).
const RISK_CONFIDENCE_FLOOR: f64 = 0.85;

/// True when the classification is confident enough to print its risk label.
pub(super) fn risk_line_visible(confidence: f64) -> bool {
    confidence >= RISK_CONFIDENCE_FLOOR
}

/// Build the `[INTENT]` block with temporal context and routing hints.
pub(super) fn build_base_context(
    intent: &kavach_chain::IntentAnalysis,
    routing: &kavach_chain::RoutingDecision,
    session: &kavach_session::SessionState,
) -> String {
    let search_year = kavach_hook::current_year().to_string();
    let search_month = kavach_hook::current_month().to_string();
    let search_week = kavach_hook::current_week().to_string();
    let confidence_str = format!("{:.2}", intent.confidence);
    let mut kvs: Vec<(&str, &str)> = vec![
        ("type", &intent.intent_type),
        ("confidence", &confidence_str),
        ("complexity", &intent.complexity),
    ];
    if risk_line_visible(intent.confidence) {
        kvs.push(("risk", &intent.risk_level));
    }
    let skill_str = routing.skill_name.clone();
    if routing.use_skill {
        kvs.push(("skill", &skill_str));
    }
    let agent_str = routing.agent_name.clone();
    if !agent_str.is_empty() {
        kvs.push(("agent", &agent_str));
    }
    if intent.requires_research && !session.research_done {
        kvs.push(("research", "REQUIRED"));
    }
    if intent.intent_type == "memory" {
        kvs.push(("memory_action", "USE kavach db write — NOT MEMORY.md files"));
    }
    kvs.push(("search_year", &search_year));
    kvs.push(("search_month", &search_month));
    kvs.push(("search_week", &search_week));
    kavach_hook::context_block("INTENT", &kvs)
}

#[cfg(test)]
mod tests {
    use super::risk_line_visible;

    #[test]
    fn low_confidence_omits_risk_line() {
        // The general-leaf default (0.5) and the old destructive leaf (0.75)
        // are guesses — neither may print a risk label.
        assert!(!risk_line_visible(0.5));
        assert!(!risk_line_visible(0.75));
    }

    #[test]
    fn confident_classifications_keep_risk_line() {
        assert!(risk_line_visible(0.85));
        assert!(risk_line_visible(0.9));
    }
}
