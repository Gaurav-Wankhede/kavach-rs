//! Safety classifier for the command following `source .env`.
//!
//! POLICY (fail-OPEN for genuine runners): the env-leak gate must NEVER hard-block
//! a real migration/deploy command just because its task-runner is not on a fixed
//! list. A runner that CONSUMES env vars silently (sqlx, cargo, just, make, npm,
//! psql, an unknown CLI) is SAFE here. The only shapes that genuinely leak a secret
//! INTO the conversation context are an EXPLICIT print of an env value
//! (`echo $X` / `printf` / `env` / `set` / `printenv` / `export`) or a raw shell
//! that can do so (`bash -c '...echo $SECRET...'`) — those, and a destructive psql,
//! stay unsafe. Everything else is allowed. (See the `FAILURE_MODE` note in the
//! parent module: this replaces the old closed binary allowlist that fail-CLOSED on
//! every unknown runner and strangled `source .env && just migrate`.)

/// Command segments whose job is to print a value/environment into context. Matched
/// at a command boundary so a piped/chained print is caught but an identifier
/// substring inside an argument is not. (`cat` is intentionally absent: reading the
/// `.env` file itself is owned by `check_dotenv_read`; `cat mig.sql` is harmless.)
const ENV_PRINTERS: &[&str] = &["echo ", "printf ", "printenv ", "env ", "set ", "export "];

/// Return true when the post-source command does NOT leak a secret into context.
/// Fail-OPEN: explicit printers + raw shells rejected; silent consumers allowed.
/// See decision.engine.env_leak_fail_open_policy.
pub(crate) fn is_safe_downstream(downstream: &str) -> bool {
    let lc = downstream.trim().to_lowercase();
    let Some(first_token) = lc.split_whitespace().next() else {
        // empty downstream loads nothing -> nothing to expose
        return true;
    };
    let basename = std::path::Path::new(first_token)
        .file_name()
        .and_then(|n| n.to_str())
        .map_or(first_token, |b| b);
    // A `cd <dir>` or `DATABASE_URL=<val>` PREFIX is harmless on its own, but it must
    // not MASK a leaky command chained after it (`cd /x; printenv`). The whole-string
    // leak classifier below already scans every `;`/`|`/`&` segment, so fall through
    // to it rather than blanket-allowing on the prefix. A bare `cd`/assignment with
    // no chained leak is allowed there (no printer segment present).
    // psql is conditionally safe: allowed only when it carries no destructive
    // SQL verb. Recognise psql as the leading binary OR anywhere in a compound
    // downstream (`echo ..; psql ..`, `psql .. | head`) — a harmless prefix/pipe
    // must not mask a safe psql; the destructive-verb classifier is the real gate.
    if basename == "psql" || invokes_psql(&lc) {
        return crate::gates::sql_destructive::destructive_sql_keyword(&lc).is_none();
    }
    // The ONLY genuinely-leaky shapes: an explicit print of an env value, or a raw
    // shell that can echo one. A secret reaches the context only through these.
    !leaks_env_value(&lc, basename)
}

/// True when the downstream EXPLICITLY prints or can print an env value into the
/// conversation context. This is the narrow, real risk — everything else is a
/// runner that consumes env silently and is allowed (fail-open).
///
/// `cat` is deliberately NOT here: `source .env && cat mig.sql` reads a SQL file,
/// not the secret. Reading the `.env` file itself is owned by `check_dotenv_read`.
fn leaks_env_value(lc: &str, basename: &str) -> bool {
    // A raw shell-with-command (or any python) can `echo $SECRET` — treat as leaky.
    if basename.starts_with("python") {
        return true;
    }
    if matches!(basename, "bash" | "sh" | "zsh" | "fish") && lc.contains("-c") {
        return true;
    }
    // Builtins whose JOB is to print values/environment, matched at a command
    // boundary (start, or after a `;`/`|`/`&` separator) so a piped/chained print
    // (`just info | echo $X`, `cd /x; printenv`) is caught, but an identifier
    // substring inside an argument is not.
    lc.split([';', '|', '&'])
        .map(str::trim)
        .any(prints_env_value)
}

/// True when a single command segment is (or begins with) a value-printing builtin.
fn prints_env_value(seg: &str) -> bool {
    // Bare dump-the-environment builtins.
    if matches!(seg, "env" | "printenv" | "export") {
        return true;
    }
    // `set` is special: BARE `set` (or `set VAR=x`) dumps/mutates the environment and
    // IS leaky, but `set -a` / `set +a` / `set -e` / `set -o pipefail` are shell-OPTION
    // toggles — the standard idiom for exporting a sourced `.env` to a child runner —
    // and must NOT be flagged. Leaky only when `set`'s first arg is not a `-`/`+` flag.
    if seg == "set" {
        return true;
    }
    if let Some(rest) = seg.strip_prefix("set ") {
        let first = rest.trim_start();
        return !(first.starts_with('-') || first.starts_with('+'));
    }
    // The remaining printers always take a (possibly `$SECRET`) argument.
    ENV_PRINTERS.iter().any(|p| seg.starts_with(p))
}

/// True when a `psql` command appears at any command boundary in a compound
/// downstream (`echo ..; psql ..`, `psql .. | head`, `a && psql ..`), so a
/// harmless prefix or pipe does not mask a safe psql. Matches `psql` only as a
/// command word (boundary-prefixed + word-terminated), never the substring of
/// another token. The destructive-SQL classifier remains the real safety gate.
fn invokes_psql(lc: &str) -> bool {
    lc.split([';', '|', '&'])
        .map(str::trim)
        .filter_map(|seg| seg.split_whitespace().next())
        .any(|tok| {
            std::path::Path::new(tok)
                .file_name()
                .and_then(|n| n.to_str())
                == Some("psql")
        })
}
