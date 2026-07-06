// env_guard.rs test suite — extracted from inline `#[cfg(test)] mod tests` to
// drop env_guard.rs under the 300-line oversized-file limit (May 2026
// split-env-guard-microservices roadmap). Included via `#[path = "env_guard_tests.rs"]`
// so tests retain `use super::*` access to crate-private items.
#![cfg(test)]
use super::*;

// check_env_sourcing tests moved to env_guard_sourcing.rs (May 2026 split).

// check_env_value_read tests
#[test]
fn blocks_printenv_with_arg() {
    assert!(check_env_value_read("printenv SECRET_KEY").is_some());
    assert!(check_env_value_read("printenv DATABASE_URL").is_some());
}

#[test]
fn allows_printenv_no_arg() {
    assert!(check_env_value_read("printenv").is_none());
    assert!(check_env_value_read("printenv | grep -o '^[^=]*'").is_none());
    // awk names-only from .env is safe
    assert!(check_env_value_read("awk -F= '/^[A-Z]/ {print $1}' .env | sort").is_none());
    // names-only then filter is safe
    assert!(check_env_value_read("env | grep -o '^[^=]*' | grep PATTERN").is_none());
}

#[test]
fn blocks_echo_dollar() {
    assert!(check_env_value_read("echo $SECRET").is_some());
    assert!(check_env_value_read("echo ${API_KEY}").is_some());
}

#[test]
fn allows_echo_no_dollar() {
    assert!(check_env_value_read("echo hello").is_none());
}

#[test]
fn blocks_env_grep() {
    assert!(check_env_value_read("env | grep SECRET").is_some());
    assert!(check_env_value_read("printenv | grep API").is_some());
}

#[test]
fn blocks_cat_dotenv() {
    assert!(check_env_value_read("cat .env").is_some());
    assert!(check_env_value_read("cat /project/.env.local").is_some());
}

#[test]
fn allows_grep_public_prefix_on_dotenv() {
    // PUBLIC_ vars are non-sensitive (browser-exposed, Astro/Vite convention)
    assert!(check_env_value_read("grep PUBLIC_ .env").is_none());
    assert!(check_env_value_read("grep '^PUBLIC_' .env").is_none());
    assert!(
        check_env_value_read(r#"grep -E "PUBLIC_(ACCOUNTS|DASHBOARD|API)_URL" .env"#).is_none()
    );
    assert!(check_env_value_read(
            r#"grep -E "^PUBLIC_(ACCOUNTS|DASHBOARD|RAINFIRE|JACOBS|SOUNDBAK|API|SITE|MAIN_SITE)_URL" .env"#
        ).is_none());
}

#[test]
fn allows_grep_framework_prefixes_on_dotenv() {
    // VITE_ (Vite), NEXT_PUBLIC_ (Next.js), REACT_APP_ (CRA), etc.
    assert!(check_env_value_read("grep VITE_ .env").is_none());
    assert!(check_env_value_read("grep NEXT_PUBLIC_ .env").is_none());
    assert!(check_env_value_read("grep REACT_APP_ .env").is_none());
    assert!(check_env_value_read("grep EXPO_PUBLIC_ .env").is_none());
    assert!(check_env_value_read("grep NUXT_PUBLIC_ .env").is_none());
    assert!(check_env_value_read(r#"grep -E "^VITE_(API_URL|SITE_NAME)" .env"#).is_none());
}

#[test]
fn blocks_grep_secret_vars_on_dotenv() {
    // Non-prefixed grep on .env still blocked
    assert!(check_env_value_read("grep SECRET .env").is_some());
    assert!(check_env_value_read("grep DATABASE_URL .env").is_some());
    assert!(check_env_value_read("env | grep API_KEY").is_some());
    // No safe prefix at all
    assert!(check_env_value_read("grep PORT .env").is_some());
    assert!(check_env_value_read("grep STRIPE .env").is_some());
}

#[test]
fn allows_unrelated_commands() {
    assert!(check_env_value_read("cargo build --release").is_none());
    assert!(check_env_value_read("git status").is_none());
    // "env" as substring of a word (env-cross, environment) must not trigger.
    assert!(check_env_value_read(
            r#"kavach db query --project nicole-carpenter --category decision 2>&1 | grep -E "phase[23]|migration|env-cross|routing-bug""#
        ).is_none());
    assert!(check_env_value_read("grep -r environment .").is_none());
}

#[test]
fn allows_bare_source_dotenv() {
    // POLICY (decision.engine.env-guard-source-load-allowed): sourcing loads values
    // into the child env, NOT into context — only a subsequent PRINT leaks. Bare
    // source is allowed; the leak-readers (cat/echo/printenv) stay blocked elsewhere.
    assert!(check_env_value_read("source .env").is_none());
    assert!(check_env_value_read("source /project/.env.local").is_none());
    // The set -a; . ./.env; set +a idiom (export sourced .env to a child) is allowed.
    assert!(check_env_value_read("set -a; . ./.env; set +a").is_none());
}

#[test]
fn allows_source_dotenv_with_safe_downstream() {
    // sqlx migrate run uses DATABASE_URL but never prints it
    assert!(
        check_env_value_read("source .env && sqlx migrate run --source migrations_local").is_none()
    );
    // kavach db pg-fix-checksum takes --dsn flag and never prints secrets
    assert!(
        check_env_value_read(
            "source .env && kavach db pg-fix-checksum --dsn $DATABASE_URL --version 36 --file x.sql"
        )
        .is_none()
    );
    // cargo test uses env without printing
    assert!(check_env_value_read("source .env && cargo test").is_none());
}

#[test]
fn blocks_source_dotenv_with_unsafe_downstream() {
    // echo $VAR after sourcing would expose values
    assert!(check_env_value_read("source .env && echo $DATABASE_URL").is_some());
}

#[test]
fn source_dotenv_then_printer_message_points_to_runtime_consume() {
    // The leak is the PRINT; the block message must steer to in-process consume.
    let msg = check_env_value_read("source .env && echo $DATABASE_URL").expect("should block");
    assert!(
        msg.contains("receipt"),
        "expected receipt-only guidance in: {msg}"
    );
    assert!(
        msg.contains("sourcing itself is fine"),
        "expected source-is-allowed framing in: {msg}"
    );
}

#[test]
fn extract_post_source_command_basic() {
    assert_eq!(
        extract_post_source_command("source .env && psql $DATABASE_URL -f file.sql"),
        Some("psql $DATABASE_URL -f file.sql".to_owned())
    );
}

#[test]
fn extract_post_source_command_semicolon() {
    assert_eq!(
        extract_post_source_command("source .env; cargo test"),
        Some("cargo test".to_owned())
    );
}

#[test]
fn extract_post_source_command_no_downstream() {
    assert_eq!(extract_post_source_command("source .env"), None);
}

#[test]
fn extract_post_source_command_dot_form() {
    assert_eq!(
        extract_post_source_command(". .env && make run"),
        Some("make run".to_owned())
    );
}

#[test]
fn extract_post_source_command_with_stderr_redirect() {
    assert_eq!(
        extract_post_source_command(
            "source ../.env 2>/dev/null; sqlx migrate info --source migrations_local"
        ),
        Some("sqlx migrate info --source migrations_local".to_owned())
    );
    assert_eq!(
        extract_post_source_command(
            "source .env 2>/dev/null && sqlx migrate run --source migrations_local"
        ),
        Some("sqlx migrate run --source migrations_local".to_owned())
    );
    assert_eq!(
        extract_post_source_command("source .env >/dev/null 2>&1 && cargo test"),
        Some("cargo test".to_owned())
    );
}

#[test]
fn allows_source_dotenv_with_redirect_and_safe_downstream() {
    // The exact pattern from the blocked command
    assert!(
        check_env_value_read(
            "source ../.env 2>/dev/null; sqlx migrate info --source migrations_local"
        )
        .is_none()
    );
    assert!(
        check_env_value_read(
            "source .env 2>/dev/null && sqlx migrate run --source migrations_local"
        )
        .is_none()
    );
}

#[test]
fn allows_absolute_path_to_sqlx() {
    // Absolute paths to sqlx must work — `~/.cargo/bin/sqlx`, `/usr/local/bin/sqlx`.
    assert!(
        check_env_value_read(
            "source .env && ~/.cargo/bin/sqlx migrate run --source migrations_local"
        )
        .is_none()
    );
    assert!(
        check_env_value_read(
            "source .env && /usr/local/bin/sqlx migrate info --source migrations_local"
        )
        .is_none()
    );
    assert!(check_env_value_read("source .env && ./target/release/sqlx migrate run").is_none());
}

#[test]
fn has_source_builtin_recognizes_command_position() {
    // Command position — must trigger
    assert!(has_source_builtin("source .env"));
    assert!(has_source_builtin("source .env && cargo test"));
    assert!(has_source_builtin("cd foo && source .env && cargo run"));
    assert!(has_source_builtin(". .env"));
}

#[test]
fn has_source_builtin_rejects_flag_argument() {
    // --source is a sqlx CLI flag, not the builtin
    assert!(!has_source_builtin(
        "sqlx migrate run --source migrations_local"
    ));
    assert!(!has_source_builtin("sqlx migrate info --source migrations"));
    // Word containing "source" must not match
    assert!(!has_source_builtin("echo opensource"));
    assert!(!has_source_builtin("ls /usr/src/source"));
}

#[test]
fn allows_sqlx_with_source_flag_when_no_dotenv() {
    // Must not trigger the .env branch at all
    assert!(check_env_value_read("sqlx migrate run --source migrations_local").is_none());
    assert!(check_env_value_read("sqlx migrate info --source migrations_local").is_none());
}

#[test]
fn allows_dot_form_with_sqlx_source_flag_regression() {
    // Regression for M-extract-skips-past-falsepos: `--source` is a false-positive hit for
    // the "source " needle. Parser must continue to the `. ./.env` command-position hit.
    assert!(
        check_env_value_read(
            "cd /tmp && . ./.env && ~/.cargo/bin/sqlx migrate run --source migrations_local"
        )
        .is_none()
    );
    assert!(
        check_env_value_read(". ./.env && sqlx migrate info --source migrations_local").is_none()
    );
    assert_eq!(
        extract_post_source_command(
            "cd /tmp && . ./.env && sqlx migrate run --source migrations_local"
        ),
        Some("sqlx migrate run --source migrations_local".to_owned())
    );
}

#[test]
fn allows_source_env_and_sqlx_with_source_flag() {
    // Real target command: source .env && sqlx migrate run --source migrations_local
    assert!(
        check_env_value_read("source .env && sqlx migrate run --source migrations_local").is_none()
    );
    assert!(
        check_env_value_read(
            "source ../.env && ~/.cargo/bin/sqlx migrate run --source migrations_local"
        )
        .is_none()
    );
}

#[test]
fn psql_is_operation_aware_after_source() {
    // Operation-aware policy: psql after `source .env` is SAFE for read/write,
    // so the env-leak gate does NOT fire (no value is echoed). A bare -f file
    // load carries no visible verb -> treated as non-destructive here; the psql
    // write-bypass gate still inspects its contents separately.
    assert!(check_env_value_read("source .env && psql -c \"SELECT version()\"").is_none());
    assert!(check_env_value_read("source .env && psql -c \"INSERT INTO t VALUES (1)\"").is_none());
    assert!(check_env_value_read("source .env && psql -f /tmp/query.sql").is_none());
    assert!(
        check_env_value_read("source .env && /usr/local/bin/psql -c \"SELECT version()\"")
            .is_none()
    );
    // A destructive verb makes psql unsafe at the env layer too (defense-in-depth).
    assert!(
        check_env_value_read("source .env && psql -c \"DELETE FROM users WHERE id=1\"").is_some()
    );
    assert!(check_env_value_read("source .env && psql -c \"DROP TABLE users\"").is_some());
}

#[test]
fn skip_shell_redirects_handles_common_forms() {
    assert_eq!(skip_shell_redirects("2>/dev/null && cmd"), "&& cmd");
    assert_eq!(skip_shell_redirects(">/dev/null 2>&1 && cmd"), "&& cmd");
    assert_eq!(skip_shell_redirects("&>/dev/null; cmd"), "; cmd");
    assert_eq!(skip_shell_redirects("&& cmd"), "&& cmd");
    assert_eq!(skip_shell_redirects("; cmd"), "; cmd");
}

#[test]
fn blocks_set_declare() {
    assert!(check_env_value_read("set").is_some());
    assert!(check_env_value_read("declare -p").is_some());
}

#[test]
fn blocks_python_environ() {
    assert!(check_env_value_read("python3 -c \"import os; print(os.environ)\"").is_some());
    assert!(
        check_env_value_read("python3 -c \"import os,json; print(json.dumps(os.environ))\"")
            .is_some()
    );
}

#[test]
fn allows_python_environ_as_subprocess_arg() {
    // os.environ['KEY'] passed to subprocess — value stays in process memory, never printed.
    assert!(check_env_value_read(
            "python3 -c \"import subprocess,os; subprocess.run(['psql', os.environ['DATABASE_URL']], input=sql, capture_output=True)\""
        ).is_none());
    assert!(check_env_value_read(
            "python3 -c \"import subprocess,os; subprocess.run(['psql', os.environ[\\\"DATABASE_URL\\\"]], capture_output=True)\""
        ).is_none());
}

#[test]
fn blocks_proc_environ() {
    assert!(check_env_value_read("cat /proc/self/environ").is_some());
}

#[test]
fn blocks_head_tail_on_dotenv() {
    assert!(check_env_value_read("head .env").is_some());
    assert!(check_env_value_read("tail -n 5 .env").is_some());
    assert!(check_env_value_read("less .env").is_some());
    assert!(check_env_value_read("strings .env").is_some());
}

// ---------- POSIX system var allowlist (FIX: contract_violation) ----------

#[test]
fn allows_printenv_for_safe_system_vars() {
    // POSIX system vars are non-secret — reading them is safe.
    assert!(check_env_value_read("printenv PATH").is_none());
    assert!(check_env_value_read("printenv HOME").is_none());
    assert!(check_env_value_read("printenv USER").is_none());
    assert!(check_env_value_read("printenv SHELL").is_none());
    assert!(check_env_value_read("printenv PWD").is_none());
    assert!(check_env_value_read("printenv LANG").is_none());
    assert!(check_env_value_read("printenv LC_ALL").is_none());
    assert!(check_env_value_read("printenv LC_CTYPE").is_none());
    assert!(check_env_value_read("printenv TZ").is_none());
    assert!(check_env_value_read("printenv TERM").is_none());
}

#[test]
fn allows_printenv_safe_var_with_pipe_chain() {
    // The original use case: `printenv PATH | tr ':' '\n'` to inspect PATH.
    assert!(check_env_value_read("printenv PATH | tr ':' '\\n'").is_none());
    assert!(check_env_value_read("printenv HOME | head -1").is_none());
}

#[test]
fn blocks_printenv_for_secret_vars_unchanged() {
    // Existing behavior preserved.
    assert!(check_env_value_read("printenv DATABASE_URL").is_some());
    assert!(check_env_value_read("printenv API_KEY").is_some());
    assert!(check_env_value_read("printenv SECRET_KEY").is_some());
    assert!(check_env_value_read("printenv JWT_SECRET").is_some());
}

#[test]
fn blocks_printenv_for_loader_injection_vars() {
    // Per openclaw GHSA-xgf2-vxv2-rrmg, loader vars are excluded from allowlist.
    assert!(check_env_value_read("printenv LD_LIBRARY_PATH").is_some());
    assert!(check_env_value_read("printenv LD_PRELOAD").is_some());
    assert!(check_env_value_read("printenv DYLD_LIBRARY_PATH").is_some());
    assert!(check_env_value_read("printenv NODE_OPTIONS").is_some());
}

#[test]
fn allows_echo_safe_system_vars() {
    assert!(check_env_value_read("echo $PATH").is_none());
    assert!(check_env_value_read("echo $HOME").is_none());
    assert!(check_env_value_read("echo ${USER}").is_none());
    assert!(check_env_value_read("echo $SHELL $HOME").is_none());
    assert!(check_env_value_read("echo $LC_ALL").is_none());
}

#[test]
fn blocks_echo_secret_vars_unchanged() {
    assert!(check_env_value_read("echo $SECRET").is_some());
    assert!(check_env_value_read("echo $DATABASE_URL").is_some());
    assert!(check_env_value_read("echo ${API_KEY}").is_some());
}

#[test]
fn blocks_echo_mixed_safe_and_secret() {
    // If ANY var in the echo command is unsafe, block.
    assert!(check_env_value_read("echo $HOME $SECRET").is_some());
    assert!(check_env_value_read("echo $PATH:$API_KEY").is_some());
}

#[test]
fn is_safe_system_var_recognizes_lc_family() {
    // LC_<anything> is POSIX-locale family — all safe.
    assert!(is_safe_system_var("LC_ALL"));
    assert!(is_safe_system_var("LC_CTYPE"));
    assert!(is_safe_system_var("LC_NAME"));
    assert!(is_safe_system_var("LC_TELEPHONE"));
    assert!(is_safe_system_var("lc_all"));
}

#[test]
fn is_safe_system_var_rejects_non_system_names() {
    assert!(!is_safe_system_var("DATABASE_URL"));
    assert!(!is_safe_system_var("API_KEY"));
    assert!(!is_safe_system_var(""));
    assert!(!is_safe_system_var("LD_LIBRARY_PATH"));
    assert!(!is_safe_system_var("DYLD_INSERT_LIBRARIES"));
    // Length cap (don't blow up on huge inputs).
    assert!(!is_safe_system_var(&"X".repeat(64)));
}

// ---------- Edge-case tests from review (Issues #4, #5) ----------

#[test]
fn blocks_echo_concatenated_safe_and_secret() {
    // $HOME$SECRET — no separator. Parser must extract HOME and SECRET as
    // two separate vars, find SECRET unsafe, block.
    assert!(check_env_value_read("echo $HOME$SECRET").is_some());
    assert!(check_env_value_read("echo $PATH$API_KEY").is_some());
}

#[test]
fn blocks_echo_param_expansion_with_secret_default() {
    // ${HOME:-$SECRET} — bash parameter expansion. Parser must extract HOME
    // (stop at colon), then continue scanning and find $SECRET, block.
    assert!(check_env_value_read("echo ${HOME:-$SECRET}").is_some());
    assert!(check_env_value_read("echo ${USER:=$DATABASE_URL}").is_some());
}

#[test]
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "shell parameter-expansion fixture: ${USER:-default} is POSIX shell syntax under test, not a Rust format arg"
)]
fn allows_echo_braced_safe_var_with_default() {
    // ${HOME:-/tmp} — default is literal, not another var. HOME is safe. Allow.
    assert!(check_env_value_read("echo ${HOME:-/tmp}").is_none());
    assert!(check_env_value_read("echo ${USER:-default}").is_none());
}

#[test]
fn echo_treats_positional_and_pid_as_no_var() {
    // $1 (positional), $$ (pid), $? (exit) have no variable name to check.
    // Parser correctly skips. echo runs only literals → no $-expansion of secrets → allow.
    assert!(check_env_value_read("echo $$").is_none());
    assert!(check_env_value_read("echo $1").is_none());
}
