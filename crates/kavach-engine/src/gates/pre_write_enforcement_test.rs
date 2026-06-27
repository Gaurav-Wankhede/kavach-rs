//! `new_crate_allowed` coverage: session flag, env=1, inline marker, negatives.
use super::package::new_crate_allowed;

#[test]
fn should_allow_new_crate_when_session_confirmed() {
    assert!(new_crate_allowed(true, None, ""));
}

#[test]
fn should_allow_new_crate_when_env_set() {
    assert!(new_crate_allowed(false, Some("1"), ""));
}

#[test]
fn should_allow_new_crate_when_marker_present() {
    assert!(new_crate_allowed(
        false,
        None,
        "# kavach: new-crate confirmed by user\n[package]"
    ));
}

#[test]
fn should_deny_new_crate_without_signal() {
    assert!(!new_crate_allowed(false, None, "[package]\nname = \"x\""));
}

#[test]
fn should_deny_new_crate_wrong_env_value() {
    assert!(!new_crate_allowed(false, Some("0"), ""));
    assert!(!new_crate_allowed(false, Some("true"), ""));
}
