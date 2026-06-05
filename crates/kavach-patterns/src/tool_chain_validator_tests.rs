use super::*;

fn tools(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn research_intent_with_edit_is_mismatch() {
    let v = validate("research", &tools(&["Read", "Grep"]), "Edit");
    assert!(v.is_some());
    assert!(v.unwrap().reason.contains("research"));
}

#[test]
fn research_intent_with_read_is_ok() {
    let v = validate("research", &tools(&["WebSearch"]), "Read");
    assert!(v.is_none());
}

#[test]
fn debug_three_edits_no_read_is_mismatch() {
    let v = validate("debug", &tools(&["Edit", "Edit", "Edit"]), "Edit");
    assert!(v.is_some(), "expected mismatch");
    assert!(v.unwrap().reason.contains("3+ consecutive Edits"));
}

#[test]
fn debug_edits_with_recent_read_is_ok() {
    let v = validate("debug", &tools(&["Edit", "Read", "Edit", "Edit"]), "Edit");
    assert!(v.is_none(), "Read inside lookback window must absolve");
}

#[test]
fn implement_websearch_after_edits_is_drift() {
    let v = validate("implement", &tools(&["Edit", "Edit", "Edit"]), "WebSearch");
    assert!(v.is_some());
    assert!(v.unwrap().reason.contains("drift"));
}

#[test]
fn implement_websearch_first_is_ok() {
    let v = validate("implement", &tools(&["Read"]), "WebSearch");
    assert!(v.is_none());
}

#[test]
fn unknown_intent_never_mismatches() {
    let v = validate("planning", &tools(&["Edit", "Edit", "Edit"]), "Edit");
    assert!(v.is_none());
}

#[test]
fn empty_recent_tools_handled() {
    let v = validate("debug", &[], "Edit");
    assert!(v.is_none());
}
