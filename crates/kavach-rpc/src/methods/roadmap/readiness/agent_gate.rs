/// Check if a card is agent-gated (not actionable by an agent).
///
/// A card is AGENT-GATED iff its body declares an external prerequisite an
/// agent can never satisfy: owner action, prod deploy, deploy-soak, browser
/// harness, or explicit `AGENT_BLOCKED:` marker. This is a SCHEDULING GATE
/// (k8s 1.26 `schedulingGates` pattern): the dispatcher must skip the card
/// entirely — exactly like an unmet dependency — so the autonomous loop never
/// claims work no agent can progress.
#[must_use]
pub fn is_agent_gated(content: &str) -> bool {
    const HARD_MARKERS: [&str; 8] = [
        "[no agent code]",
        "ZERO agent-executable code",
        "OWNER-TASK",
        "OWNER-GATED",
        "OWNER-ONLY",
        "UMBRELLA",
        "EPIC",
        "child-derived",
    ];
    if HARD_MARKERS.iter().any(|m| content.contains(m)) {
        return true;
    }
    content
        .lines()
        .any(|raw| raw.trim().starts_with("AGENT_BLOCKED:"))
}
