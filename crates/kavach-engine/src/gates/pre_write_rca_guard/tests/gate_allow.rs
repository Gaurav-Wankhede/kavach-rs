//! `check` allow-path tests: RCA present, session flag, exempt paths, low risk,
//! non-gated intents/tools, and the bypass/bulk-sweep env escapes.
use super::super::bypass::{BULK_SWEEP_ENV, BYPASS_ENV};
use super::super::check;

#[test]
fn allows_when_rca_block_present() {
    let msg = "Analyzing the issue.\n[RCA]\nsymptom: ...\nwhy5: ...";
    assert!(check("Edit", "debug", "high", msg, false, "src/lib.rs").is_none());
}

#[test]
fn allows_when_rca_with_dash_header() {
    let msg = "[RCA — ordering bug]\nsymptom: stop hook bypass";
    assert!(check("Edit", "implement", "medium", msg, false, "src/lib.rs").is_none());
}

#[test]
fn allows_when_session_rca_persisted_across_turns() {
    assert!(check("Edit", "implement", "medium", "", true, "src/lib.rs").is_none());
}

#[test]
fn allows_claude_md_config_without_rca() {
    assert!(
        check(
            "Edit",
            "implement",
            "high",
            "",
            false,
            "/Users/x/.claude/CLAUDE.md"
        )
        .is_none()
    );
}

#[test]
fn allows_json_settings_without_rca() {
    assert!(check("Edit", "implement", "high", "", false, "settings.json").is_none());
}

#[test]
fn allows_root_claude_md_without_rca() {
    assert!(check("Edit", "implement", "high", "", false, "CLAUDE.md").is_none());
}

#[test]
fn allows_global_claude_md_without_rca() {
    assert!(
        check(
            "Edit",
            "implement",
            "high",
            "",
            false,
            "/Users/x/.claude/CLAUDE.md"
        )
        .is_none()
    );
}

#[test]
fn allows_low_risk_without_rca() {
    assert!(check("Edit", "implement", "low", "fix typo", false, "src/lib.rs").is_none());
}

#[test]
fn allows_non_implement_intent() {
    assert!(
        check(
            "Edit",
            "general",
            "high",
            "no rca needed",
            false,
            "src/lib.rs"
        )
        .is_none()
    );
    assert!(
        check(
            "Edit",
            "explain",
            "high",
            "explanatory",
            false,
            "src/lib.rs"
        )
        .is_none()
    );
}

#[test]
fn allows_non_edit_tool() {
    assert!(check("Read", "debug", "high", "no rca", false, "src/lib.rs").is_none());
    assert!(check("Bash", "debug", "high", "no rca", false, "src/lib.rs").is_none());
}

#[test]
fn bypass_env_var_allows_when_set() {
    let result = temp_env::with_var(BYPASS_ENV, Some("1"), || {
        check("Edit", "implement", "medium", "", false, "src/lib.rs")
    });
    assert!(result.is_none(), "bypass should allow even without [RCA]");
}

#[test]
fn bulk_sweep_env_allows_when_set() {
    let result = temp_env::with_var(BULK_SWEEP_ENV, Some("bulk.lint-sweep-001"), || {
        check("Edit", "implement", "high", "", false, "src/lib.rs")
    });
    assert!(
        result.is_none(),
        "active bulk sweep should authorize edit without per-edit [RCA]"
    );
}
