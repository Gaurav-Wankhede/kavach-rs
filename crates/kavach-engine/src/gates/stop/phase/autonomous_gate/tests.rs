use super::stop_gate_fires;

#[test]
fn auto_mode_fires() {
    assert!(stop_gate_fires("auto"));
}

#[test]
fn bypass_permissions_fires() {
    assert!(stop_gate_fires("bypassPermissions"));
}

#[test]
fn plan_does_not_fire() {
    assert!(!stop_gate_fires("plan"));
}

#[test]
fn default_does_not_fire() {
    assert!(!stop_gate_fires("default"));
}

#[test]
fn accept_edits_does_not_fire() {
    assert!(!stop_gate_fires("acceptEdits"));
}

#[test]
fn dont_ask_does_not_fire() {
    assert!(!stop_gate_fires("dontAsk"));
}

#[test]
fn empty_mode_does_not_fire() {
    assert!(!stop_gate_fires(""));
}
