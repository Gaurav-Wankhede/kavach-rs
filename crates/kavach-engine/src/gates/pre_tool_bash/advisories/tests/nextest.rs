//! nextest advisory + scaffold tests: suggest-on-plain-test, skip-when-nextest,
//! quoted-arg exemption, and the idempotent/fail-soft config scaffold.
use super::super::{check_nextest_advisory, scaffold_nextest_config};

#[test]
fn should_suggest_nextest_for_plain_cargo_test() {
    assert!(check_nextest_advisory("cargo test --workspace").is_some());
}

#[test]
fn should_not_suggest_nextest_when_already_using_it() {
    assert!(check_nextest_advisory("cargo nextest run").is_none());
}

#[test]
fn nextest_advisory_ignores_phrase_in_quoted_arg() {
    // CWE-184: the phrase inside another tool's quoted arg is data.
    assert!(check_nextest_advisory(r"rg -n 'x|cargo test' src/").is_none());
    assert!(check_nextest_advisory(r#"grep "cargo test" build.log"#).is_none());
    assert!(check_nextest_advisory(r#"echo "use cargo test then""#).is_none());
    assert!(check_nextest_advisory("cargo test --workspace").is_some());
    assert!(check_nextest_advisory(r#"echo "skip" && cargo test -p x"#).is_some());
}

#[test]
fn scaffold_writes_config_when_absent_in_rust_project() {
    let dir = std::env::temp_dir().join(format!("kv_scaffold_a_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("mk dir");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("cargo.toml");
    let r = scaffold_nextest_config("cargo test --workspace", &dir);
    assert!(r.is_some(), "must scaffold when config absent");
    assert!(dir.join(".config/nextest.toml").is_file(), "file written");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn scaffold_skips_when_config_already_present() {
    let dir = std::env::temp_dir().join(format!("kv_scaffold_b_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join(".config")).expect("mk .config");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("cargo.toml");
    std::fs::write(dir.join(".config/nextest.toml"), "# user's own\n").expect("existing");
    let r = scaffold_nextest_config("cargo test", &dir);
    assert!(r.is_none(), "must not overwrite an existing config");
    let kept = std::fs::read_to_string(dir.join(".config/nextest.toml")).unwrap();
    assert_eq!(kept, "# user's own\n", "existing file left byte-identical");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn scaffold_skips_non_rust_directory() {
    let dir = std::env::temp_dir().join(format!("kv_scaffold_c_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("mk dir");
    assert!(scaffold_nextest_config("cargo test", &dir).is_none());
    assert!(!dir.join(".config").exists(), "no .config created");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn scaffold_ignores_non_test_command() {
    let dir = std::env::temp_dir().join(format!("kv_scaffold_d_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("mk dir");
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("cargo.toml");
    assert!(scaffold_nextest_config("cargo build", &dir).is_none());
    assert!(scaffold_nextest_config(r#"echo "cargo test""#, &dir).is_none());
    std::fs::remove_dir_all(&dir).ok();
}
