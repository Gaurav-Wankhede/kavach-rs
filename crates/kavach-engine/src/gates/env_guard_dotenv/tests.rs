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
fn safe_downstream_rejects_python_and_bash() {
    // python (global ban) and an arbitrary `bash -c` (can echo secrets) stay unsafe.
    assert!(!is_safe_downstream("python -c 'print(env)'"));
    assert!(!is_safe_downstream("bash -c 'echo $SECRET'"));
}

#[test]
fn safe_downstream_allows_just_make_and_unknown_runners() {
    // THE FIX: a task-runner that consumes env silently is SAFE here, even if it is
    // not on any fixed list. `just` (the omission that strangled `source .env && just
    // migrate`), `task`, `mise`, `dotenv`, and any unknown runner all pass — the gate
    // must not hard-block a genuine migration command.
    assert!(is_safe_downstream("just migrate"));
    assert!(is_safe_downstream("just migrate-info"));
    assert!(is_safe_downstream("task db:migrate"));
    assert!(is_safe_downstream("mise run migrate"));
    assert!(is_safe_downstream("dotenv -- sqlx migrate run"));
    assert!(is_safe_downstream("some-unknown-runner deploy"));
}

#[test]
fn safe_downstream_runner_with_pager_pipe_stays_safe() {
    // Piping a SAFE runner's output to a pager must NOT flip it to unsafe — the
    // pager shows the runner's stdout (migration status), never the raw env value.
    assert!(is_safe_downstream("just migrate-info | tail -8"));
    assert!(is_safe_downstream("sqlx migrate info 2>&1 | tail"));
    assert!(is_safe_downstream("cargo run | head -20"));
}

#[test]
fn safe_downstream_rejects_explicit_env_value_print() {
    // The narrow REAL risk: a command whose job is to print the secret into context.
    assert!(!is_safe_downstream("echo $DATABASE_URL"));
    assert!(!is_safe_downstream("printf '%s' \"$SECRET\""));
    assert!(!is_safe_downstream("printenv"));
    assert!(!is_safe_downstream("env"));
    assert!(!is_safe_downstream("set"));
    // ...and a leaky print hiding behind a safe-looking prefix/pipe.
    assert!(!is_safe_downstream("just info | echo $DATABASE_URL"));
    assert!(!is_safe_downstream("cd /x; printenv"));
}

#[test]
fn safe_downstream_allows_cat_of_non_env_file() {
    // `source .env && cat mig.sql` reads a SQL file, not the secret — allowed.
    // (Reading the .env file itself is owned by the separate check_dotenv_read gate.)
    assert!(is_safe_downstream("cat migrations_local/341_step_up.sql"));
}

#[test]
fn safe_downstream_allows_set_shell_option_toggles() {
    // `set -a` / `set +a` / `set -e` / `set -o pipefail` are shell-OPTION toggles —
    // the standard idiom for exporting a sourced .env to a child runner — NOT an
    // environment dump. They must NOT be flagged (regression: blocked the exact
    // `set -a && . ./.env && set +a && cargo run …` migration command).
    assert!(is_safe_downstream("set -a && . ./.env && set +a && cargo run --bin x"));
    assert!(is_safe_downstream("set -e; cargo build"));
    assert!(is_safe_downstream("set +a && just migrate"));
    assert!(is_safe_downstream("set -o pipefail; sqlx migrate run"));
    // ...but a BARE `set` (or piped to a pager) still dumps the env — stays blocked.
    assert!(!is_safe_downstream("set"));
    assert!(!is_safe_downstream("set | grep DATABASE"));
}

#[test]
fn safe_downstream_psql_is_operation_aware() {
    // psql consuming a DSN for a read/write op is safe — it never echoes the env.
    assert!(is_safe_downstream("psql $DATABASE_URL -c 'SELECT 1'"));
    assert!(is_safe_downstream("psql $DATABASE_URL -c 'UPDATE t SET x=1'"));
    assert!(is_safe_downstream("psql $DATABASE_URL"));
    // A destructive verb makes it unsafe at this layer (and the psql gate blocks it).
    assert!(!is_safe_downstream("psql $DATABASE_URL -c 'DELETE FROM t'"));
    assert!(!is_safe_downstream("psql $DATABASE_URL -c 'DROP TABLE t'"));
    assert!(!is_safe_downstream("psql $DATABASE_URL -c 'TRUNCATE t'"));
}

#[test]
fn safe_downstream_psql_after_harmless_prefix_or_pipe() {
    // A psql that is not the LEADING binary — behind an echo prefix or a pipe —
    // must still be recognised as safe (no destructive verb present).
    assert!(is_safe_downstream("echo hi; psql $DATABASE_URL -c 'SELECT 1'"));
    assert!(is_safe_downstream("psql $DATABASE_URL -f mig.sql | head"));
    assert!(is_safe_downstream("cd /x; psql $DATABASE_URL -c 'UPDATE t SET y=1'"));
}

#[test]
fn safe_downstream_compound_psql_still_blocks_destructive() {
    // The destructive-verb guard must fire even when psql is behind a prefix/pipe —
    // a harmless prefix must NOT mask a DROP/DELETE/TRUNCATE.
    assert!(!is_safe_downstream("echo go; psql $DATABASE_URL -c 'DROP TABLE t'"));
    assert!(!is_safe_downstream("psql $DATABASE_URL -c 'DELETE FROM t' | tee log"));
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
