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
        "BLOCKED: `printenv VAR` exposes the secret value. \
         Variable names are visible — values are not. \
         Use `printenv | rg -o '^[^=]*'` to list names only (toolbelt: rg). \
         POSIX system vars (PATH, HOME, USER, SHELL, PWD, LANG, ...) are allowed."
            .into(),
    )
}

/// `echo $VAR` / `echo ${VAR}` — expands and prints (all-safe-vars exempt).
pub(crate) fn check_echo(lc: &str) -> Option<String> {
    if !(lc.starts_with("echo ") && lc.contains('$') && !echo_only_references_safe_vars(lc)) {
        return None;
    }
    Some(
        "BLOCKED: `echo $VAR` exposes the secret value. \
         Reference the variable name in code without expanding it. \
         POSIX system vars (PATH, HOME, USER, SHELL, PWD, LANG, ...) are allowed."
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
        "BLOCKED: `env | grep PATTERN` exposes secret values. \
         To check which variables are present, use: \
         `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` (toolbelt: rg is 5-13x faster than awk) (reads names from file) or \
         `env | grep -o '^[^=]*' | grep PATTERN` (filters names only from environment)."
            .into(),
    )
}

/// `set` / `declare` — dumps all shell variables including secrets.
pub(crate) fn check_set_dump(lc: &str) -> Option<String> {
    let hit = lc == "set"
        || lc == "declare"
        || lc == "declare -p"
        || lc.starts_with("set ")
        || lc.starts_with("declare ");
    hit.then(|| {
        "BLOCKED: `set`/`declare` dumps all shell variables including secret values. \
         Use `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` (toolbelt: rg is 5-13x faster than awk) to list names only."
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
