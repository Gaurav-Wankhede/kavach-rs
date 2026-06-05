use super::*;

#[test]
fn test_forbidden_phrases() {
    let g = ResearchGate::new();
    let violations = g.check_forbidden_phrases("I think this is fine");
    assert!(!violations.is_empty());
}

#[test]
fn test_research_gate_require() {
    let g = ResearchGate::new();
    let r = g.require_research("implement axum server");
    assert!(r.is_some());
    assert!(r.unwrap().mandatory);
}

#[test]
fn test_validate_research_done() {
    let g = ResearchGate::new();
    assert!(g.validate_research_done("Used WebSearch to find patterns"));
    assert!(!g.validate_research_done("just guessing"));
}
