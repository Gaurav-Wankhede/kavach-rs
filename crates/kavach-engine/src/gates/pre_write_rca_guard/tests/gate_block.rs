//! `check` block-path tests: gated intents/risks without an RCA must block, and
//! the negative case of the bulk-sweep escape.
use super::super::bypass::BULK_SWEEP_ENV;
use super::super::check;

#[test]
fn gates_nested_claude_md_as_normal_file() {
    // Security: a nested src/.../CLAUDE.md must NOT be exempt — only
    // project-root and global config CLAUDE.md skip the RCA gate.
    assert!(
        check(
            "Edit",
            "implement",
            "high",
            "",
            false,
            "src/feature/CLAUDE.md"
        )
        .is_some()
    );
}

#[test]
fn bulk_sweep_env_empty_string_does_not_activate() {
    let result = temp_env::with_var(BULK_SWEEP_ENV, Some(""), || {
        check("Edit", "implement", "high", "", false, "src/lib.rs")
    });
    assert!(
        result.is_some(),
        "empty BULK_SWEEP_ENV must not authorize — gate must still demand RCA"
    );
}

#[test]
fn session_flag_alone_controls_gate_permit() {
    assert!(check("Edit", "implement", "medium", "", true, "src/lib.rs").is_none());
    assert!(check("Edit", "implement", "medium", "", false, "src/lib.rs").is_some());
    assert!(check("Write", "debug", "high", "", true, "src/lib.rs").is_none());
    assert!(check("Write", "debug", "high", "", false, "src/lib.rs").is_some());
}

#[test]
fn blocks_implement_medium_without_rca() {
    let result = check(
        "Edit",
        "implement",
        "medium",
        "let me edit this file",
        false,
        "src/lib.rs",
    );
    assert!(result.is_some());
    let reason = result.unwrap_or_default();
    assert!(reason.contains("[RCA_FIRST]"));
    assert!(reason.contains("implement"));
    assert!(reason.contains("medium"));
}

#[test]
fn blocks_debug_high_without_rca() {
    assert!(
        check(
            "Edit",
            "debug",
            "high",
            "fixing the bug",
            false,
            "src/lib.rs"
        )
        .is_some()
    );
}

#[test]
fn blocks_refactor_critical_without_rca() {
    assert!(
        check(
            "Write",
            "refactor",
            "critical",
            "rewriting module",
            false,
            "src/lib.rs"
        )
        .is_some()
    );
}

#[test]
fn blocks_empty_message_when_session_flag_false() {
    assert!(check("Edit", "debug", "high", "", false, "src/lib.rs").is_some());
}

#[test]
fn blocked_message_uses_rca_first_vocabulary() {
    let result = check("Edit", "debug", "high", "", false, "src/lib.rs");
    let reason = result.unwrap_or_default();
    assert!(reason.contains("[RCA_FIRST]"));
    assert!(reason.contains("why-chain→root_cause"));
    assert!(reason.contains("→ retry"));
}
