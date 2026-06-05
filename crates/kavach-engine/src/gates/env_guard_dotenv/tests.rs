//! Safe-downstream allowlist (sqlx/cargo/kavach allowed; python/psql/bash rejected;
//! `DATABASE_URL=` + cd + path-basename handling) and `.env*` filename detection.
use super::downstream::is_safe_downstream;
use super::filename::detect_env_filename;

#[test]
fn safe_downstream_recognizes_sqlx() {
    assert!(is_safe_downstream("sqlx migrate run"));
}

#[test]
fn safe_downstream_recognizes_cargo_npm_bun() {
    assert!(is_safe_downstream("cargo run"));
    assert!(is_safe_downstream("npm run dev"));
    assert!(is_safe_downstream("bun run start"));
}

#[test]
fn safe_downstream_recognizes_kavach_subcommands() {
    assert!(is_safe_downstream(
        "kavach db pg-fix-checksum --dsn $DATABASE_URL --version 5"
    ));
}

#[test]
fn safe_downstream_rejects_python_psql_bash() {
    assert!(!is_safe_downstream("python -c 'print(env)'"));
    assert!(!is_safe_downstream("psql $DATABASE_URL"));
    assert!(!is_safe_downstream("bash -c 'echo $SECRET'"));
}

#[test]
fn safe_downstream_recognizes_database_url_assignment() {
    assert!(is_safe_downstream(
        "DATABASE_URL=postgres://x sqlx migrate run"
    ));
}

#[test]
fn safe_downstream_recognizes_cd() {
    assert!(is_safe_downstream("cd /project && cargo run"));
}

#[test]
fn safe_downstream_handles_path_basename() {
    assert!(is_safe_downstream("/usr/local/bin/cargo run"));
}

#[test]
fn detect_env_filename_recognizes_local() {
    assert_eq!(detect_env_filename("cat .env.local"), ".env.local");
}

#[test]
fn detect_env_filename_recognizes_envrc() {
    assert_eq!(detect_env_filename("source .envrc"), ".envrc");
}

#[test]
fn detect_env_filename_falls_back_to_dotenv() {
    assert_eq!(detect_env_filename("cat .env"), ".env");
    assert_eq!(detect_env_filename("ls"), ".env");
}

#[test]
fn detect_env_filename_handles_custom_suffix() {
    assert_eq!(detect_env_filename("cat .env.custom"), ".env.custom");
}
