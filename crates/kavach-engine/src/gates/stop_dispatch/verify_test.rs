//! Witness-machinery tests (sibling of witness.rs per §`NANO_FILE`; `super` is the
//! `witness` module). decision.kavach.verify-rs-nanofile-split-2026-06-17.
use super::{
    WitnessRun, discover_rust_workspace, failing_witness_report, is_rust_workspace,
    verify_command_env, witness_root_from_card,
};

#[test]
fn failing_witness_report_names_command_and_echoes_stderr() {
    // rca.opaque-witness: a failed witness MUST surface which command failed and
    // its compiler output, or the agent cannot tell `check` from `clippy`.
    let report = failing_witness_report(
        "cargo clippy --workspace --all-targets -- -D warnings",
        "error: deref which would be done by auto-deref\n",
    );
    assert!(report.contains("clippy"), "names the failing command");
    assert!(
        report.contains("auto-deref"),
        "echoes the real compiler error"
    );
    assert!(
        report.contains("[WITNESS_FAILED]"),
        "carries the agent-facing tag"
    );
}

#[test]
fn non_cargo_dir_is_not_rust() {
    // rca.keystone-trap: a non-Rust dir must NOT be a witness FAILURE.
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(!is_rust_workspace(dir.path()));
}

#[test]
fn cargo_dir_is_rust_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("write");
    assert!(is_rust_workspace(dir.path()));
}

#[test]
fn discover_finds_workspace_in_subdir_monorepo() {
    // rca.monorepo-verify-blind: discovery walks one subdir level.
    let root = tempfile::tempdir().expect("tempdir");
    let backend = root.path().join("Backend");
    std::fs::create_dir(&backend).expect("mkdir");
    std::fs::write(backend.join("Cargo.toml"), "[workspace]\n").expect("write");
    assert_eq!(discover_rust_workspace(root.path()), Some(backend));
}

#[test]
fn discover_prefers_root_when_root_is_workspace() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("Cargo.toml"), "[workspace]\n").expect("write");
    assert_eq!(
        discover_rust_workspace(root.path()),
        Some(root.path().to_path_buf())
    );
}

#[test]
fn discover_returns_none_for_non_rust_tree() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(root.path().join("docs")).expect("mkdir");
    assert_eq!(discover_rust_workspace(root.path()), None);
}

#[test]
fn spawn_error_failed_unprovable_are_distinct() {
    assert_ne!(WitnessRun::SpawnError, WitnessRun::Failed);
    assert_ne!(WitnessRun::SpawnError, WitnessRun::Unprovable);
    assert_ne!(WitnessRun::Failed, WitnessRun::Unprovable);
    assert_eq!(WitnessRun::Passed, WitnessRun::Passed);
}

#[test]
fn verify_command_env_absent_returns_none() {
    temp_env::with_var_unset("KAVACH_VERIFY_CMD", || {
        assert_eq!(verify_command_env(), None);
    });
}

#[test]
fn verify_command_env_returns_value_when_set() {
    temp_env::with_var("KAVACH_VERIFY_CMD", Some("echo hello"), || {
        assert_eq!(verify_command_env(), Some("echo hello".to_owned()));
    });
}

#[test]
fn witness_root_absent_yields_none() {
    let content = "title: a card\nDEPENDS_ON: other\nbody text\n";
    assert_eq!(witness_root_from_card(content), None);
}

#[test]
fn witness_root_extracted_and_trimmed() {
    let content = "title\nWITNESS_ROOT:   /Users/x/kavach-rs  \nmore\n";
    assert_eq!(
        witness_root_from_card(content),
        Some("/Users/x/kavach-rs".to_owned())
    );
}

#[test]
fn witness_root_first_declaration_wins() {
    let content = "WITNESS_ROOT: /first\nWITNESS_ROOT: /second\n";
    assert_eq!(witness_root_from_card(content), Some("/first".to_owned()));
}

#[test]
fn witness_root_empty_value_is_ignored() {
    // A bare `WITNESS_ROOT:` with no path is not a usable hint → None.
    assert_eq!(witness_root_from_card("WITNESS_ROOT:   \n"), None);
}
