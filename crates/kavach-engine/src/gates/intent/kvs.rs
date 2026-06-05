//! Build the `[INTENT]` key-value block: classification, temporal context,
//! and skill/agent routing hints.

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
        ("risk", &intent.risk_level),
    ];
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
