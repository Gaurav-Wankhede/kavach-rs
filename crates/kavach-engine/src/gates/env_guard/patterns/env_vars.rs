//! Environment-variable leak patterns: printenv, echo, env|grep, set/declare,
//! python os.environ dump, and `/proc/self/environ`.
use super::util::is_names_only;
use crate::gates::env_guard_safelist::{echo_only_references_safe_vars, is_safe_system_var};
use crate::gates::env_guard_shell_parse::first_word_is;

/// `printenv VAR` — dumps a named value (POSIX system vars exempt).
pub(crate) fn check_printenv(lc: &str) -> Option<String> {
    let after = lc.trim_start_matches("printenv").trim_start();
    if !(lc.starts_with("printenv ") && !after.is_empty() && !after.starts_with('|')) {
        return None;
    }
    let var_name = after
        .split(|c: char| c.is_whitespace() || c == '|' || c == ';' || c == '&')
        .next()
        .unwrap_or("");
    if is_safe_system_var(var_name) {
        return None;
    }
    Some(
        "[SECRET_CONSUME] `printenv VAR` would print the secret into context. NOT a hand-back. \
         If you need the value to DO the task: WRITE a runtime script (Rust first: \
         `std::env::var(\"VAR\")` / `dotenvy`) that reads it INSIDE its own process, runs the \
         operation (migration / deletion / query), and prints ONLY a pass/fail receipt — never \
         the value. Run it yourself. \
         To see only NAMES: `printenv | rg -o '^[^=]*'`. POSIX system vars are allowed -> retry."
            .into(),
    )
}

/// `echo $VAR` / `echo ${VAR}` — expands and prints (all-safe-vars exempt).
pub(crate) fn check_echo(lc: &str) -> Option<String> {
    if !(lc.starts_with("echo ") && lc.contains('$') && !echo_only_references_safe_vars(lc)) {
        return None;
    }
    Some(
        "[SECRET_CONSUME] `echo $VAR` would print the secret into context. NOT a hand-back. \
         If you need the value to DO the task: consume it INSIDE a runtime script (Rust first: \
         `std::env::var` / `dotenvy`) that runs the operation and prints ONLY a receipt — never \
         the value. Reference the NAME in code, never expand it to stdout. Run it yourself. \
         POSIX system vars are allowed -> retry."
            .into(),
    )
}

/// `env | grep PATTERN` / `printenv | grep PATTERN` — filters but prints values.
pub(crate) fn check_env_grep(lc: &str) -> Option<String> {
    if (!first_word_is(lc, "env") && !lc.contains("printenv"))
        || !lc.contains("grep")
        || is_names_only(lc)
    {
        return None;
    }
    Some(
        "[SECRET_CONSUME] `env | grep PATTERN` exposes secret values -> to check which \
         variables are present, use: \
         `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` (toolbelt: rg is 5-13x faster than awk) (reads names from file) or \
         `env | grep -o '^[^=]*' | grep PATTERN` (filters names only from environment) -> retry."
            .into(),
    )
}

/// `set` / `declare` — dumps all shell variables including secrets.
pub(crate) fn check_set_dump(lc: &str) -> Option<String> {
    // `set -a`/`set +a`/`set -e`/`set -o pipefail` are shell-OPTION toggles — the
    // standard idiom for exporting a sourced .env to a child — and dump NOTHING.
    // Only BARE `set` (or `set VAR=x`) dumps the environment. Checked per command
    // segment so a chained bare `set` is still caught. Mirrors the downstream
    // classifier (env_guard_dotenv/downstream.rs::prints_env_value).
    let set_is_dump = lc.split([';', '|', '&', '\n']).map(str::trim).any(|seg| {
        seg == "set"
            || seg
                .strip_prefix("set ")
                .is_some_and(|rest| !matches!(rest.trim_start().chars().next(), Some('-' | '+')))
    });
    let hit = set_is_dump || lc == "declare" || lc == "declare -p" || lc.starts_with("declare ");
    hit.then(|| {
        "[SECRET_CONSUME] `set`/`declare` dumps all shell variables including secret values \
         -> use `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` (toolbelt: rg is 5-13x faster than awk) to list names only -> retry."
            .into()
    })
}

/// `print(os.environ)` / `json.dumps(os.environ)` — Python env dump.
pub(crate) fn check_python_environ(lc: &str) -> Option<String> {
    let has_python = lc.contains("python") || lc.contains("python3");
    let has_environ = lc.contains("os.environ") || lc.contains("os.getenv");
    let dumps = lc.contains("print(os.environ")
        || lc.contains("print(os.getenv")
        || lc.contains("json.dumps(os.environ")
        || lc.contains("str(os.environ")
        || lc.contains("pprint(os.environ")
        || (lc.contains("os.environ)")
            && !lc.contains("os.environ['")
            && !lc.contains("os.environ[\""));
    (has_python && has_environ && dumps).then(|| {
        "BLOCKED: `os.environ`/`os.getenv` exposes secret values. \
         Read variable names with `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` (toolbelt: rg is 5-13x faster than awk) instead."
            .into()
    })
}

/// `/proc/self/environ` — Linux process environment dump.
pub(crate) fn check_proc_environ(lc: &str) -> Option<String> {
    (lc.contains("/proc/self/environ") || lc.contains("/proc/1/environ")).then(|| {
        "BLOCKED: `/proc/self/environ` exposes all process environment variables. \
         Use `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` (toolbelt: rg is 5-13x faster than awk) to list names only."
            .into()
    })
}
