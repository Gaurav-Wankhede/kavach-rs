// Shared fixtures for the compiler test leaves + the legacy Pattern-6 proofs.
// THE ORACLE for backward-compat: the default harness stays byte-compatible
// with the pre-enhancement loop template, and the round-trip-to-disk works.
use crate::cmd::goal::compile::{compile_to_workflow, to_workflow_js};
use crate::cmd::goal::loop_yaml::{GoalLoopYaml, Harness};

pub(super) fn sample() -> GoalLoopYaml {
    GoalLoopYaml::test_exit_code(
        "goal-paseto-introspect",
        "Wire paseto.rs -> introspect",
        "cargo nextest run -p kavach-rpc introspect",
    )
}

pub(super) fn with_harness(h: Harness) -> GoalLoopYaml {
    let mut g = sample();
    g.harness = h;
    g
}

#[test]
fn default_harness_emits_loop_until_done() {
    let js = to_workflow_js(&sample());
    assert!(js.contains("name: 'goal-loop'"), "missing loop meta:\n{js}");
    assert!(js.contains("title: 'Work'"));
    assert!(js.contains("title: 'Verify'"));
    assert!(js.contains("title: 'Diagnose'"));
}

#[test]
fn loop_embeds_oracle_and_guard() {
    let js = to_workflow_js(&sample());
    assert!(js.contains("cargo nextest run -p kavach-rpc introspect"));
    assert!(js.contains("attempt < MAX && budget.remaining() > BUDGET_FLOOR"));
    assert!(js.contains("kavach db event --type goal_loop_attempt"));
}

#[test]
fn loop_fans_out_declared_lenses() {
    let js = to_workflow_js(&sample());
    assert!(js.contains("'lens-0'") && js.contains("'lens-2'"));
    assert!(!js.contains("'lens-3'"));
}

#[test]
fn injection_in_intent_is_escaped() {
    let g = GoalLoopYaml::test_exit_code("g", "break ' out \" now", "true");
    let js = to_workflow_js(&g);
    assert!(
        js.contains(r#"const INTENT = "break ' out \" now""#),
        "got:\n{js}"
    );
}

#[test]
fn compile_writes_workflow_js_to_disk() {
    let dir = std::env::temp_dir().join(format!("kavach-compile-{}", std::process::id()));
    let rel = compile_to_workflow(&sample(), &dir).expect("compile");
    let abs = dir.join(&rel);
    let js = std::fs::read_to_string(&abs).expect("read back");
    assert!(js.contains("export const meta ="));
    drop(std::fs::remove_dir_all(&dir));
}
