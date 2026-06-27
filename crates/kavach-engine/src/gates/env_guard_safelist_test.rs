//! Safe-var recognition (POSIX basics + LC_* prefix), unsafe rejection, and
//! echo-scan concatenation / shell-builtin handling.
use super::echo::echo_only_references_safe_vars;
use super::is_safe_system_var;

#[test]
fn safe_var_recognizes_posix_basics() {
    assert!(is_safe_system_var("HOME"));
    assert!(is_safe_system_var("PATH"));
    assert!(is_safe_system_var("LANG"));
}

#[test]
fn safe_var_recognizes_lc_family_prefix() {
    assert!(is_safe_system_var("LC_TIME"));
    assert!(is_safe_system_var("LC_FOO"));
    assert!(is_safe_system_var("lc_messages"));
}

#[test]
fn safe_var_rejects_known_unsafe_names() {
    assert!(!is_safe_system_var("DATABASE_URL"));
    assert!(!is_safe_system_var("LD_PRELOAD"));
    assert!(!is_safe_system_var("NODE_OPTIONS"));
}

#[test]
fn safe_var_rejects_too_long_names() {
    assert!(!is_safe_system_var(&"A".repeat(33)));
}

#[test]
fn echo_safe_when_only_safe_vars() {
    assert!(echo_only_references_safe_vars("echo $home $path"));
}

#[test]
fn echo_unsafe_when_unknown_var_present() {
    assert!(!echo_only_references_safe_vars("echo $home $myvar"));
}

#[test]
fn echo_handles_concatenation_correctly() {
    // peek+next must not swallow the $ for the second var.
    assert!(!echo_only_references_safe_vars("echo $home$myvar"));
}

#[test]
fn echo_safe_with_only_shell_builtins() {
    // $$, $1, $? are positional/special — not env vars; allowed.
    assert!(echo_only_references_safe_vars("echo $$ $1 $?"));
}
