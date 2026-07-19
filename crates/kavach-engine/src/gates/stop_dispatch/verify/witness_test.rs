use super::{WitnessRun, discover_rust_workspace, is_rust_workspace, run_workspace_witnesses};
use std::io::Write;

fn tmpdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir creation should succeed")
}

#[test]
fn verify_cmd_runs_and_passes() {
    // Per-call command overrides the dispatch CWD, so it works even inside the
    // kavach-rs Rust workspace.
    assert_eq!(
        run_workspace_witnesses(None, Some("exit 0")),
        WitnessRun::Passed
    );
}

#[test]
fn verify_cmd_failure_is_failed() {
    assert_eq!(
        run_workspace_witnesses(None, Some("exit 1")),
        WitnessRun::Failed
    );
}

#[test]
fn per_call_verify_cmd_overrides_env() {
    temp_env::with_var("KAVACH_VERIFY_CMD", Some(std::ffi::OsStr::new("exit 1")), || {
        // Per-call command wins and passes.
        assert_eq!(
            run_workspace_witnesses(None, Some("exit 0")),
            WitnessRun::Passed
        );
    });
}

#[test]
fn witness_root_env_takes_precedence_over_verify_cmd() {
    let tmp = tmpdir();
    let ws = tmp.path().join("backend");
    std::fs::create_dir(&ws).expect("create backend dir");
    std::fs::File::create(ws.join("Cargo.toml"))
        .expect("create Cargo.toml")
        .write_all(b"[package]\nname = 'x'")
        .expect("write Cargo.toml");
    temp_env::with_var("WITNESS_ROOT", Some(ws.as_os_str()), || {
        // WITNESS_ROOT points to a Rust workspace, so cargo witnesses run even
        // though a verify_cmd is supplied. Minimal manifest fails cargo check.
        let run = run_workspace_witnesses(None, Some("exit 0"));
        assert!(matches!(run, WitnessRun::Failed | WitnessRun::SpawnError));
    });
}

#[test]
fn per_card_root_wins_over_env() {
    let tmp = tmpdir();
    let ws = tmp.path().join("backend");
    std::fs::create_dir(&ws).expect("create backend dir");
    std::fs::File::create(ws.join("Cargo.toml"))
        .expect("create Cargo.toml")
        .write_all(b"[package]\nname = 'x'")
        .expect("write Cargo.toml");
    let other = tmpdir();
    temp_env::with_var("WITNESS_ROOT", Some(other.path().as_os_str()), || {
        let run = run_workspace_witnesses(Some(ws.to_str().expect("utf8 path")), Some("exit 0"));
        assert!(matches!(run, WitnessRun::Failed | WitnessRun::SpawnError));
    });
}

#[test]
fn is_rust_workspace_detects_cargo_toml() {
    let tmp = tmpdir();
    assert!(!is_rust_workspace(tmp.path()));
    std::fs::File::create(tmp.path().join("Cargo.toml")).expect("create Cargo.toml");
    assert!(is_rust_workspace(tmp.path()));
}

#[test]
fn discover_rust_workspace_finds_immediate_child() {
    let tmp = tmpdir();
    let child = tmp.path().join("backend");
    std::fs::create_dir(&child).expect("create backend dir");
    std::fs::File::create(child.join("Cargo.toml")).expect("create Cargo.toml");
    assert_eq!(discover_rust_workspace(tmp.path()), Some(child));
}
