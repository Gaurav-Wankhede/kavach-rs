//! Witness-machinery tests (sibling of witness.rs per §`MICRO_FILE`; `super` is the
//! `witness` module). decision.kavach.verify-rs-microfile-split-2026-06-17.
use super::{WitnessRun, discover_rust_workspace, is_rust_workspace, verify_command_env};

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
