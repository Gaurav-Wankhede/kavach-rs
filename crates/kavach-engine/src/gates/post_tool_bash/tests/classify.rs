//! Output/command classifier tests: test command, empty suite, package, port.
use crate::gates::post_tool_bash::detect::{
    classify_kavach_misuse, detect_port_conflict, is_empty_test_suite, is_package_install,
    is_package_not_found, is_test_command, KavachMisuse,
};

#[test]
fn test_detect_cargo_test() {
    assert!(is_test_command("cargo test --workspace"));
    assert!(is_test_command("cargo nextest run"));
}

#[test]
fn test_detect_bun_test() {
    assert!(is_test_command("bun test"));
    assert!(is_test_command("bun run test"));
    assert!(is_test_command("bunx playwright test"));
    assert!(is_test_command("bunx --bun playwright test"));
}

#[test]
fn test_detect_npx_test() {
    assert!(is_test_command("npx playwright test"));
    assert!(is_test_command("npx jest --coverage"));
}

#[test]
fn test_not_test_command() {
    assert!(!is_test_command("cargo build"));
    assert!(!is_test_command("git status"));
}

#[test]
fn test_empty_test_suite_detected() {
    let output = "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out";
    assert!(is_empty_test_suite(Some(output)));
}

#[test]
fn test_non_empty_test_suite_ok() {
    let output = "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out";
    assert!(!is_empty_test_suite(Some(output)));
}

#[test]
fn test_empty_test_suite_none_output() {
    assert!(!is_empty_test_suite(None));
    assert!(!is_empty_test_suite(Some("")));
}

#[test]
fn test_package_install_detected() {
    assert!(is_package_install("bun add @envelop/depth-limit"));
    assert!(is_package_install("npm install express"));
    assert!(is_package_install("cargo add serde"));
    assert!(!is_package_install("cargo build"));
}

#[test]
fn test_package_not_found() {
    assert!(is_package_not_found(Some(
        "error: GET https://registry.npmjs.org/foo - 404"
    )));
    assert!(is_package_not_found(Some("No matching version found")));
    assert!(!is_package_not_found(Some("added 3 packages")));
    assert!(!is_package_not_found(None));
}

#[test]
fn test_port_conflict_eaddrinuse() {
    let output = "Error: listen EADDRINUSE: address already in use :::9247";
    assert_eq!(detect_port_conflict(Some(output)), Some(9247));
}

#[test]
fn test_port_conflict_localhost() {
    let output = "EADDRINUSE: address already in use 127.0.0.1:3000";
    assert_eq!(detect_port_conflict(Some(output)), Some(3000));
}

#[test]
fn test_port_conflict_port_in_use() {
    let output = "error: port 8080 is already in use";
    assert_eq!(detect_port_conflict(Some(output)), Some(8080));
}

#[test]
fn test_port_conflict_none() {
    assert_eq!(detect_port_conflict(None), None);
    assert_eq!(detect_port_conflict(Some("")), None);
    assert_eq!(
        detect_port_conflict(Some("Server started on port 3000")),
        None
    );
}

#[test]
fn test_kavach_misuse_unknown_verb() {
    let out = "error: unrecognized subcommand 'doctorr'\n\ntip: a similar subcommand exists: 'doctor'";
    assert_eq!(classify_kavach_misuse("kavach doctorr", Some(out)), Some(KavachMisuse::UnknownVerb));
}

#[test]
fn test_kavach_misuse_unknown_flag() {
    let out = "error: unexpected argument '--qualified-name' found";
    assert_eq!(classify_kavach_misuse("kavach db write --qualified-name x", Some(out)), Some(KavachMisuse::UnknownFlag));
}

#[test]
fn test_kavach_misuse_stale_binary() {
    let out = "error: unrecognized subcommand 'commands'";
    assert_eq!(classify_kavach_misuse("kavach commands --tree", Some(out)), Some(KavachMisuse::StaleBinary));
}

#[test]
fn test_kavach_misuse_negatives() {
    assert_eq!(classify_kavach_misuse("git status", Some("error: unrecognized subcommand 'x'")), None);
    assert_eq!(classify_kavach_misuse("kavach status", Some("ok: project ready")), None);
    assert_eq!(classify_kavach_misuse("kavach db get", Some("error: DB connection refused")), None);
    assert_eq!(classify_kavach_misuse("kavach doctor", None), None);
}
