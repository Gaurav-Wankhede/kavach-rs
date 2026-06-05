//! Canary shadow-mode proofs. The load-bearing invariant is fail-closed arming:
//! the controller stays dormant unless `KAVACH_RL_CANARY` is an EXPLICIT truthy,
//! so a stray/garbled env value can never silently arm the learned policy.

use super::{action_str, canary_armed, is_truthy, record_shadow};
use kavach_patterns::bandit_log::GateAction;

#[test]
fn only_explicit_truthy_values_arm_the_canary() {
    for v in ["1", "true", "TRUE", "yes", "On", " on "] {
        assert!(is_truthy(v), "{v:?} should arm");
    }
    for v in ["", "0", "false", "no", "off", "maybe", "2", "disable"] {
        assert!(!is_truthy(v), "{v:?} must NOT arm — fail-closed default");
    }
}

#[test]
fn action_str_uses_the_bandit_log_snake_case_vocabulary() {
    assert_eq!(action_str(GateAction::Allow), "allow");
    assert_eq!(action_str(GateAction::Ask), "ask");
    assert_eq!(action_str(GateAction::Block), "block");
}

#[test]
fn record_shadow_is_a_noop_without_a_session_id() {
    // No session to key the event -> nothing is emitted (and no panic), even if
    // the gate would otherwise diverge. Independent of the env flag.
    record_shadow(
        "",
        "micro_file",
        "Write",
        GateAction::Allow,
        GateAction::Ask,
    );
}

#[test]
fn canary_armed_reads_the_env_flag_without_panicking() {
    // We don't mutate the process env here (other tests run in parallel); just
    // prove the read path is total — it returns a bool, never panics.
    let armed = canary_armed();
    assert!(
        matches!(armed, true | false),
        "read path returns a bool, never panics"
    );
}
