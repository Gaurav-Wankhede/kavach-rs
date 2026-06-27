use super::*;

#[test]
fn test_run_start_default() {
    let input = HookInput {
        agent_id: "test-agent".into(),
        agent_type: "Explore".into(),
        ..Default::default()
    };
    run_start(&input).expect("test setup");
}

#[test]
fn rules_contract_covers_the_core_kavach_laws() {
    let c = SUBAGENT_RULES_CONTRACT;
    assert!(c.contains("[SUBAGENT_RULES]"));
    assert!(c.contains("TDD"), "must carry the TDD law");
    assert!(
        c.contains("let _") && c.contains(".ok()"),
        "must forbid suppression"
    );
    assert!(
        c.contains("Toolbelt") || c.contains("rg/fd"),
        "must carry toolbelt"
    );
    assert!(c.contains("RCA"), "must carry RCA-before-fix");
    assert!(c.contains("3-witness"), "must demand 3-witness evidence");
    assert!(
        c.contains("do NOT spawn further subagents"),
        "one-level fan-out: a worker is a doer, not an orchestrator"
    );
}

#[test]
fn rules_contract_is_executor_shaped_not_orchestrator() {
    let c = SUBAGENT_RULES_CONTRACT;
    assert!(
        c.contains("STOP and report") || c.contains("report the blocker"),
        "a blocked worker reports the blocker — it must NOT loop or fabricate"
    );
    assert!(
        c.contains("orphan"),
        "must forbid orphan test/handler files left to satisfy a gate"
    );
    assert!(
        c.contains("building") || c.contains("compile") || c.contains("revert"),
        "must demand the tree is left building (revert your change if it can't compile)"
    );
    assert!(
        c.contains("orchestrator") && c.contains("research"),
        "research/design is the orchestrator's — the worker implements, never WebSearch"
    );
}
