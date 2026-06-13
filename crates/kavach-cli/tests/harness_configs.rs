//! Guard the shipped multi-harness install configs against rot.
//!
//! These templates are what a user pastes into Cursor / Codex / Claude Code, so a
//! malformed file or a reference to a gate name kavach doesn't expose would break
//! a real install silently. These tests parse each config and assert every
//! `kavach gates <name>` it references is a gate the CLI actually serves.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use std::path::PathBuf;

/// Gate names the CLI dispatches (mirror of `cmd/gates.rs::print_gate_info`).
/// If a template references a name absent here, the install would no-op or error.
const VALID_GATES: &[&str] = &[
    "pre-write",
    "pre-implementation",
    "post-write",
    "pre-tool",
    "post-tool",
    "intent",
    "subagent-start",
    "subagent-stop",
    "session-start",
    "session-end",
    "pre-compact",
    "stop",
    "post-tool-failure",
    "permission",
    "permission-request",
    "notification",
    "message-display",
    "teammate-idle",
    "task-completed",
];

fn templates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/harness")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(templates_dir().join(name))
        .unwrap_or_else(|e| panic!("read {name}: {e}"))
}

/// Every `kavach gates <name>` reference across all configs names a real gate.
#[test]
fn every_referenced_gate_name_is_one_the_cli_serves() {
    for file in [
        "cursor.hooks.json",
        "codex.config.toml",
        "claude.settings.json",
    ] {
        let body = read(file);
        for chunk in body.split("kavach gates ").skip(1) {
            let gate = chunk
                .split_whitespace()
                .next()
                .expect("a gate name follows");
            assert!(
                VALID_GATES.contains(&gate),
                "{file} references unknown gate '{gate}'"
            );
        }
    }
}

#[test]
fn cursor_and_claude_configs_are_valid_json() {
    for file in ["cursor.hooks.json", "claude.settings.json"] {
        let v: serde_json::Value =
            serde_json::from_str(&read(file)).unwrap_or_else(|e| panic!("{file} not JSON: {e}"));
        assert!(v.get("hooks").is_some(), "{file} must have a hooks object");
    }
}

#[test]
fn cursor_uses_its_native_camelcase_events() {
    let v: serde_json::Value = serde_json::from_str(&read("cursor.hooks.json")).unwrap();
    let hooks = v["hooks"].as_object().expect("hooks object");
    // Cursor's real event names — not Claude Code's PascalCase.
    assert!(
        hooks.contains_key("beforeShellExecution"),
        "must use Cursor's native event"
    );
    assert!(hooks.contains_key("beforeSubmitPrompt"));
    assert!(
        hooks["preToolUse"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|e| e["command"].as_str())
            .is_some_and(|c| c.contains("pre-write")),
        "Write tools must route through pre-write"
    );
    assert!(
        !hooks.contains_key("PreToolUse"),
        "must NOT use Claude Code event names"
    );
}

/// The per-harness global-rule files ship and carry the kavach DB contract, so a
/// Cursor/Codex agent is governed by the same rules as Claude Code's CLAUDE.md.
#[test]
fn global_rule_files_ship_and_reference_the_db_contract() {
    for file in ["AGENTS.md", "kavach.mdc"] {
        let body = read(file);
        assert!(
            body.contains("kavach db write"),
            "{file} must teach the DB write contract"
        );
        assert!(
            body.contains("three-witness")
                || body.contains("three witnesses")
                || body.contains("git diff --stat"),
            "{file} must carry the evidence/verify rule"
        );
    }
    // Cursor's rule file must declare itself always-applied (its enforcement knob).
    assert!(
        read("kavach.mdc").contains("alwaysApply: true"),
        "cursor rule must alwaysApply"
    );
}

#[test]
fn every_cursor_and_codex_command_carries_its_vendor_flag() {
    let cursor = read("cursor.hooks.json");
    for chunk in cursor.split("kavach gates ").skip(1) {
        let line = chunk.lines().next().unwrap_or("");
        assert!(
            line.contains("--vendor cursor"),
            "cursor cmd missing flag: {line}"
        );
    }
    let codex = read("codex.config.toml");
    for chunk in codex.split("kavach gates ").skip(1) {
        let line = chunk.lines().next().unwrap_or("");
        assert!(
            line.contains("--vendor codex"),
            "codex cmd missing flag: {line}"
        );
    }
}
