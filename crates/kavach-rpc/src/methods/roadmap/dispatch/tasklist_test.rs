//! Unit tests for the Claude Code `TaskList` census source.
//!
//! Each test writes fixture `<id>.json` files into a temp store laid out as the
//! real one (`<root>/<scope>/<id>.json` + `<scope>/.archived-completed/`), then
//! asserts the runnable/blocked split. No network, no global env mutation
//! (env writes are unsound under threads in Rust 2024 — path resolution is
//! tested via the pure `resolve_root`).

use super::*;

/// Build a `<root>/<scope>/` store dir under a unique temp path. The unique
/// suffix is derived from `tag` (NOT a clock/random, which are banned in this
/// workspace's deterministic-test posture) so parallel tests never collide.
fn scratch_store(tag: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("kavach-tasklist-test-{tag}"));
    let scope = root.join("scope");
    // Clean any prior run so the fixture set is exactly what each test writes.
    // A NotFound error (no prior run) is the expected, acceptable outcome.
    drop(std::fs::remove_dir_all(&root));
    std::fs::create_dir_all(&scope).expect("create scope dir");
    (root, scope)
}

fn write_task(scope: &std::path::Path, id: &str, status: &str, blocked_by: &[&str]) {
    let deps = blocked_by
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"id":"{id}","subject":"t{id}","description":"d","activeForm":"a","status":"{status}","blocks":[],"blockedBy":[{deps}]}}"#
    );
    std::fs::write(scope.join(format!("{id}.json")), json).expect("write task json");
}

#[test]
fn open_tasks_count_as_runnable() {
    let (root, scope) = scratch_store("open-runnable");
    write_task(&scope, "1", "pending", &[]);
    write_task(&scope, "2", "in_progress", &[]);
    write_task(&scope, "3", "completed", &[]); // terminal → not runnable

    let (runnable, blocked) = tasklist_census(&root);
    assert_eq!(runnable, 2, "pending + in_progress are runnable, completed is not");
    assert_eq!(blocked, 0, "no deps → nothing blocked");
}

#[test]
fn task_blocked_by_open_prereq_is_blocked() {
    let (root, scope) = scratch_store("blocked");
    write_task(&scope, "1", "pending", &[]); // the prereq, still open
    write_task(&scope, "2", "pending", &["1"]); // waits on open #1

    let (runnable, blocked) = tasklist_census(&root);
    assert_eq!(runnable, 2, "both are open work");
    assert_eq!(blocked, 1, "#2 waits on still-open #1");
}

#[test]
fn task_blocked_by_completed_prereq_is_unblocked() {
    let (root, scope) = scratch_store("unblocked");
    write_task(&scope, "1", "completed", &[]); // prereq done
    write_task(&scope, "2", "pending", &["1"]); // dep satisfied

    let (runnable, blocked) = tasklist_census(&root);
    assert_eq!(runnable, 1, "only #2 is open");
    assert_eq!(blocked, 0, "#2's prereq #1 is completed → unblocked");
}

#[test]
fn dangling_blockedby_does_not_strand_task() {
    let (root, scope) = scratch_store("dangling");
    // #2 waits on #99, which does not exist in the store (archived/deleted).
    // A dangling pointer must resolve as SATISFIED, never permanently block.
    write_task(&scope, "2", "pending", &["99"]);

    let (runnable, blocked) = tasklist_census(&root);
    assert_eq!(runnable, 1, "#2 is open");
    assert_eq!(blocked, 0, "unknown prereq is treated as satisfied, not blocking");
}

#[test]
fn missing_store_contributes_zero() {
    let root = std::env::temp_dir().join("kavach-tasklist-test-absent-xyz");
    drop(std::fs::remove_dir_all(&root)); // ensure it does not exist (NotFound is fine)
    let (runnable, blocked) = tasklist_census(&root);
    assert_eq!((runnable, blocked), (0, 0), "absent store fails closed to zero");
}

#[test]
fn malformed_json_file_is_skipped_not_fatal() {
    let (root, scope) = scratch_store("malformed");
    write_task(&scope, "1", "pending", &[]);
    std::fs::write(scope.join("2.json"), b"{not valid json").expect("write garbage");

    let (runnable, blocked) = tasklist_census(&root);
    assert_eq!(runnable, 1, "the one valid open task counts; garbage is skipped");
    assert_eq!(blocked, 0, "no deps");
}

#[test]
fn archived_completed_subdir_is_not_descended() {
    let (root, scope) = scratch_store("archive-skip");
    write_task(&scope, "1", "pending", &[]);
    let archive = scope.join(".archived-completed");
    std::fs::create_dir_all(&archive).expect("create archive dir");
    // A stray runnable-looking file in the archive must NOT be counted.
    write_task(&archive, "900", "pending", &[]);

    let (runnable, _blocked) = tasklist_census(&root);
    assert_eq!(runnable, 1, "archive subdir is skipped; only the live task counts");
}

#[test]
fn unknown_status_string_fails_closed() {
    assert!(!is_runnable_cc_status("garbage"), "non-canonical status is never runnable");
    assert!(is_runnable_cc_status("pending"));
    assert!(is_runnable_cc_status("in_progress"));
    assert!(!is_runnable_cc_status("completed"));
}

#[test]
fn override_dir_wins_over_home() {
    // Pure resolution — no global env mutation (unsound under threads in Rust
    // 2024). The explicit override beats HOME.
    let resolved = resolve_root(
        Some(std::ffi::OsString::from("/tmp/kavach-override-probe")),
        Some(PathBuf::from("/home/someone")),
    )
    .expect("override always yields a root");
    assert_eq!(resolved, PathBuf::from("/tmp/kavach-override-probe"));
}

#[test]
fn home_derives_root_when_no_override() {
    let resolved = resolve_root(None, Some(PathBuf::from("/home/someone")))
        .expect("home yields a root");
    assert_eq!(resolved, PathBuf::from("/home/someone/.claude/tasks"));
}

#[test]
fn no_inputs_resolves_none() {
    assert!(resolve_root(None, None).is_none(), "both absent → fail-closed None");
}
