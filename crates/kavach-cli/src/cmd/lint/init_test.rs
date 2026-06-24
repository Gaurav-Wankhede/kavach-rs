use super::*;
use crate::cmd::lint::detect::{detect, Stack};

fn tmp(name: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("kavach-lint-test-{name}"));
    drop(std::fs::remove_dir_all(&base));
    std::fs::create_dir_all(&base).expect("mkdir tmp");
    base
}

#[test]
fn no_stack_is_noop_success() {
    let dir = tmp("empty");
    assert_eq!(run(&dir, false), 0);
}

#[test]
fn detect_finds_rust_by_cargo_toml() {
    let dir = tmp("rust-detect");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    assert_eq!(detect(&dir), vec![Stack::Rust]);
}

#[test]
fn rust_append_is_idempotent() {
    let dir = tmp("rust-append");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    assert_eq!(run(&dir, false), 0);
    let after = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(after.contains("[workspace.lints.rust]"));
    assert!(after.contains("unsafe_code = \"forbid\""));
    // Second run must NOT duplicate the table.
    assert_eq!(run(&dir, false), 0);
    let twice = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert_eq!(twice.matches("[workspace.lints.rust]").count(), 1);
}

#[test]
fn ts_writes_strict_tsconfig_when_absent() {
    let dir = tmp("ts-write");
    std::fs::write(dir.join("package.json"), "{}\n").unwrap();
    assert_eq!(run(&dir, false), 0);
    let cfg = std::fs::read_to_string(dir.join("tsconfig.json")).unwrap();
    assert!(cfg.contains("\"strict\": true"));
    assert!(cfg.contains("noUncheckedIndexedAccess"));
}

#[test]
fn existing_tsconfig_is_left_unchanged() {
    let dir = tmp("ts-keep");
    std::fs::write(dir.join("package.json"), "{}\n").unwrap();
    std::fs::write(dir.join("tsconfig.json"), "{\"mine\":1}\n").unwrap();
    assert_eq!(run(&dir, false), 0);
    let cfg = std::fs::read_to_string(dir.join("tsconfig.json")).unwrap();
    assert_eq!(cfg, "{\"mine\":1}\n");
}

#[test]
fn go_writes_golangci_when_absent() {
    let dir = tmp("go-write");
    std::fs::write(dir.join("go.mod"), "module x\n").unwrap();
    assert_eq!(run(&dir, false), 0);
    let cfg = std::fs::read_to_string(dir.join(".golangci.yml")).unwrap();
    assert!(cfg.contains("staticcheck"));
    assert!(cfg.contains("errcheck"));
}

#[test]
fn dry_run_writes_nothing() {
    let dir = tmp("dry");
    std::fs::write(dir.join("go.mod"), "module x\n").unwrap();
    assert_eq!(run(&dir, true), 0);
    assert!(!dir.join(".golangci.yml").exists());
}
