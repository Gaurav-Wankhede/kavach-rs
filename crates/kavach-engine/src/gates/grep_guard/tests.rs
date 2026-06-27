//! Grep-guard behavior: non-grep pass-through, reminder vs performance-block,
//! and anchor-aware command matching (pgrep/egrep/fgrep/zgrep/ripgrep ignored).
use super::check_grep_command;

#[test]
fn non_grep_passes() {
    assert!(check_grep_command("cargo build").is_none());
}

#[test]
fn simple_grep_gets_reminder() {
    let r = check_grep_command("grep foo bar.txt");
    assert!(r.is_some());
    assert!(r.unwrap().contains("GREP_TOOL_REMINDER"));
}

#[test]
fn recursive_grep_no_exclusions_blocks() {
    let r = check_grep_command(r#"grep -r "pattern" /home/project"#);
    assert!(r.is_some());
    assert!(r.unwrap().contains("GREP_PERFORMANCE_BLOCK"));
}

#[test]
fn recursive_grep_with_all_flags_passes() {
    let cmd = "grep -r --binary-files=without-match \
               --exclude-dir=.git --exclude-dir=target \
               --exclude-dir=node_modules --include='*.rs' pat src/";
    assert!(check_grep_command(cmd).is_none());
}

// Regression tests: anchor-aware command matching
// BUG: substring match falsely flagged pgrep/egrep/fgrep/zgrep/ripgrep
// FIX: split_whitespace().next() enforces word boundary
#[test]
fn pgrep_does_not_trigger() {
    // pgrep is a process-grep, NOT the text grep we want to redirect
    assert!(check_grep_command("pgrep node").is_none());
    assert!(check_grep_command("pgrep -f myapp").is_none());
}

#[test]
fn egrep_does_not_trigger() {
    assert!(check_grep_command("egrep pattern file.txt").is_none());
}

#[test]
fn fgrep_does_not_trigger() {
    assert!(check_grep_command("fgrep literal file.txt").is_none());
}

#[test]
fn zgrep_does_not_trigger() {
    assert!(check_grep_command("zgrep pattern file.gz").is_none());
}

#[test]
fn ripgrep_does_not_trigger() {
    // The toolbelt's own rg should NEVER be flagged
    assert!(check_grep_command("ripgrep foo").is_none());
    assert!(check_grep_command("rg foo").is_none());
}

#[test]
fn absolute_path_grep_triggers() {
    // /usr/bin/grep should still trigger (path-stripped)
    let r = check_grep_command("/usr/bin/grep foo bar.txt");
    assert!(r.is_some(), "absolute path grep should still be detected");
}

#[test]
fn recursive_grep_symbol_appends_origin_hint() {
    let cmd = "grep -r RoleQuery /src";
    let r = check_grep_command(cmd);
    assert!(r.is_some());
    let msg = r.unwrap();
    assert!(msg.contains("GREP_PERFORMANCE_BLOCK"));
    assert!(msg.contains("KAVACH_ORIGIN_HINT"));
    assert!(msg.contains("RoleQuery"));
}

#[test]
fn recursive_grep_non_symbol_no_hint() {
    let cmd = "grep -r \"pattern with spaces\" /src";
    let r = check_grep_command(cmd);
    assert!(r.is_some());
    let msg = r.unwrap();
    assert!(msg.contains("GREP_PERFORMANCE_BLOCK"));
    assert!(!msg.contains("KAVACH_ORIGIN_HINT"));
}
