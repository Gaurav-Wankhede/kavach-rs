use super::*;

#[test]
fn suffix_with_workflow_path_emits_run_workflow() {
    let s = format_suffix("worker-critic", "/tmp/wf.js", "none");
    assert!(s.contains("[AUTO_CONTINUE] run Workflow /tmp/wf.js"));
    assert!(s.contains("HARNESS [worker-critic]"));
    assert!(s.contains("last verdict: none"));
}

#[test]
fn suffix_without_workflow_path_directs_compile_first() {
    let s = format_suffix("loop-until-done", "", "pending");
    assert!(s.contains("kavach goal compile"));
    assert!(!s.contains("run Workflow "));
    assert!(s.contains("last verdict: pending"));
}

#[test]
fn suffix_carries_verdict_for_retry_decision() {
    let s = format_suffix("generate-filter", "/x/workflow.js", "fail");
    assert!(s.contains("last verdict: fail"));
    assert!(s.contains("do NOT hand-execute"));
}
