//! Auto-allow coverage: safe tools, kavach CLI, safe/unsafe rm targets, `run`.
use kavach_types::HookInput;

use crate::gates::permission_request::allow::{
    is_kavach_command, is_safe_auto_allow, is_safe_rm_target,
};
use crate::gates::permission_request::run;

#[test]
fn test_safe_tools() {
    assert!(is_safe_auto_allow("Read"));
    assert!(is_safe_auto_allow("Grep"));
    assert!(is_safe_auto_allow("WebSearch"));
    assert!(!is_safe_auto_allow("Bash"));
    assert!(!is_safe_auto_allow("Write"));
}

#[test]
fn test_kavach_command() {
    assert!(is_kavach_command("kavach db query --project foo"));
    assert!(is_kavach_command("kavach gates pre-write --hook"));
    assert!(!is_kavach_command("npm test"));
}

#[test]
fn test_safe_rm_targets() {
    assert!(is_safe_rm_target("rm -rf node_modules"));
    assert!(is_safe_rm_target("rm -rf .vite"));
    assert!(is_safe_rm_target("rm -rf .next"));
    assert!(is_safe_rm_target("rm -rf .astro"));
    assert!(is_safe_rm_target("rm -r node_modules/.vite"));
    assert!(is_safe_rm_target("rm -rf /Users/foo/node_modules"));
    assert!(is_safe_rm_target("rm -rf packages/dashboard/.astro"));
    assert!(is_safe_rm_target("rm -rf target/debug"));
    assert!(is_safe_rm_target("rm -rf __pycache__"));
}

#[test]
fn test_unsafe_rm_not_autoallowed() {
    assert!(!is_safe_rm_target("rm -rf src"));
    assert!(!is_safe_rm_target("rm -rf /etc/hosts"));
    assert!(!is_safe_rm_target("rm important_file.rs"));
    assert!(!is_safe_rm_target("cargo build"));
    // "build"/"dist" as substrings should NOT auto-allow.
    assert!(!is_safe_rm_target("rm -rf /important/dist_backup"));
    assert!(!is_safe_rm_target("rm -rf /etc/important_build_config"));
    assert!(!is_safe_rm_target("rm -rf /home/user/.cache_important"));
}

#[test]
fn test_safe_rm_exact_basenames() {
    assert!(is_safe_rm_target("rm -rf dist"));
    assert!(is_safe_rm_target("rm -rf build"));
    assert!(is_safe_rm_target("rm -rf ./dist"));
    assert!(is_safe_rm_target("rm -rf packages/app/build"));
}

#[test]
fn test_permission_request_default() {
    let input = HookInput {
        tool_name: "Read".into(),
        ..Default::default()
    };
    run(&input).expect("permission_request run should not fail");
}
