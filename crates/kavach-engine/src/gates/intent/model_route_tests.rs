use super::recommend;

#[test]
fn complex_turn_on_sonnet_recommends_opus() {
    let out = recommend("claude-sonnet-5", "complex", "low").expect("should recommend");
    assert!(out.contains("recommend:opus"));
    assert!(out.contains("/model opus"));
}

#[test]
fn high_risk_turn_on_sonnet_recommends_opus() {
    let out = recommend("claude-sonnet-5", "moderate", "high").expect("should recommend");
    assert!(out.contains("recommend:opus"));
}

#[test]
fn simple_turn_on_opus_recommends_sonnet_downgrade() {
    let out = recommend("claude-opus-4-8", "simple", "low").expect("should recommend");
    assert!(out.contains("recommend:sonnet"));
    assert!(out.contains("/model sonnet"));
}

#[test]
fn moderate_turn_on_opus_recommends_sonnet_downgrade() {
    let out = recommend("claude-opus-4-8", "moderate", "low").expect("should recommend");
    assert!(out.contains("recommend:sonnet"));
}

#[test]
fn complex_turn_already_on_opus_is_silent() {
    assert!(recommend("claude-opus-4-8", "complex", "high").is_none());
}

#[test]
fn simple_turn_already_on_sonnet_is_silent() {
    assert!(recommend("claude-sonnet-5", "simple", "low").is_none());
}

#[test]
fn empty_or_unknown_model_never_routes() {
    assert!(recommend("", "complex", "high").is_none());
    assert!(recommend("gpt-4", "complex", "high").is_none());
}
