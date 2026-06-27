//! Contract registry lookups + read-only flag correctness.
use super::get_contract;

#[test]
fn known_agents_have_contracts() {
    assert!(get_contract("research-director").is_some());
    assert!(get_contract("backend-engineer").is_some());
    assert!(get_contract("unknown").is_none());
}

#[test]
fn read_only_flag_correct() {
    assert!(get_contract("research-director").unwrap().read_only);
    assert!(!get_contract("backend-engineer").unwrap().read_only);
}

#[test]
fn injects_brainos_and_return_contract() {
    let brain = super::context::BrainContext {
        project: "kavach-rs",
        phase: "BUILD",
        doer_model: "haiku",
    };
    let lines = super::context::build_agent_context(
        "fix the gate",
        get_contract("backend-engineer"),
        &brain,
    )
    .join("\n");
    assert!(lines.contains("[AGENT_BRAINOS] project=kavach-rs phase=BUILD"));
    assert!(lines.contains("[AGENT_SPAWN] spawn a DYNAMIC subagent"));
    assert!(lines.contains("smallest doer model: haiku"));
    assert!(lines.contains("[AGENT_RETURN_CONTRACT] persist every settled decision"));
}

#[test]
fn read_only_agent_gets_structured_return_contract() {
    let brain = super::context::BrainContext {
        project: "kavach-rs",
        phase: "PLAN",
        doer_model: "composer-2.5",
    };
    let lines =
        super::context::build_agent_context("audit", get_contract("research-director"), &brain)
            .join("\n");
    assert!(lines.contains("[AGENT_RETURN_CONTRACT] return a structured result"));
    assert!(lines.contains("smallest doer model: composer-2.5"));
}

#[test]
fn empty_project_omits_brainos_line() {
    let brain = super::context::BrainContext {
        project: "",
        phase: "",
        doer_model: "",
    };
    let lines = super::context::build_agent_context("x", None, &brain).join("\n");
    assert!(!lines.contains("[AGENT_BRAINOS]"));
    assert!(!lines.contains("[AGENT_SPAWN]"));
    assert!(lines.contains("[AGENT_RESEARCH]"));
    assert!(lines.contains("NEVER trust training weights"));
}
