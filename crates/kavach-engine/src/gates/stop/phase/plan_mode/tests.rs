use super::is_plan_mode;

#[test]
fn plan_mode_string_is_plan() {
    assert!(is_plan_mode("plan"));
}

#[test]
fn auto_default_is_not_plan() {
    assert!(!is_plan_mode("default"));
}

#[test]
fn bypass_permissions_is_not_plan() {
    assert!(!is_plan_mode("bypassPermissions"));
}

#[test]
fn accept_edits_is_not_plan() {
    assert!(!is_plan_mode("acceptEdits"));
}

#[test]
fn dont_ask_is_not_plan() {
    assert!(!is_plan_mode("dontAsk"));
}

#[test]
fn empty_mode_is_not_plan() {
    assert!(!is_plan_mode(""));
}
