//! TDD: per-vendor headless argv builders. Contracts researched 2026-06-24:
//! cc `claude -p` (code.claude.com/docs/en/headless), codex `codex exec`
//! (developers.openai.com/codex/hooks), opencode/gemini non-interactive run.
use super::*;

fn req(role: AgentRole) -> VendorRequest {
    VendorRequest {
        role,
        prompt: "do the thing".into(),
        project: "kavach-rs".into(),
        max_turns: 3,
    }
}

#[test]
fn cc_argv_uses_print_mode_and_bypass() {
    let a = cc_argv(&req(AgentRole::Worker));
    assert_eq!(a[0], "claude");
    assert!(a.iter().any(|s| s == "-p"));
    assert!(a.iter().any(|s| s == "do the thing"));
    assert!(a.windows(2).any(|w| w == ["--permission-mode", "bypassPermissions"]));
    assert!(a.windows(2).any(|w| w == ["--max-turns", "3"]));
}

#[test]
fn codex_argv_uses_exec_subcommand() {
    let a = codex_argv(&req(AgentRole::Thinker));
    assert_eq!(a[0], "codex");
    assert_eq!(a[1], "exec");
    assert!(a.iter().any(|s| s == "do the thing"));
}

#[test]
fn opencode_argv_is_noninteractive_run() {
    let a = opencode_argv(&req(AgentRole::Worker));
    assert_eq!(a[0], "opencode");
    assert!(a.iter().any(|s| s == "run"));
    assert!(a.iter().any(|s| s == "do the thing"));
}

#[test]
fn gemini_argv_uses_prompt_flag() {
    let a = gemini_argv(&req(AgentRole::Worker));
    assert_eq!(a[0], "gemini");
    assert!(a.windows(2).any(|w| w[0] == "-p" && w[1] == "do the thing"));
}

#[test]
fn backend_id_matches_vendor() {
    assert_eq!(CommandBackend::cc().id(), "cc");
    assert_eq!(CommandBackend::codex().id(), "codex");
    assert_eq!(CommandBackend::opencode().id(), "opencode");
    assert_eq!(CommandBackend::gemini().id(), "gemini");
}

#[test]
fn nonzero_exit_is_failclosed() {
    // A backend whose program is `false` exits non-zero -> Err, never silent Ok.
    let b = CommandBackend {
        vendor: "cc",
        argv: |_| vec!["false".into()],
    };
    assert!(b.dispatch(&req(AgentRole::Worker)).is_err());
}
