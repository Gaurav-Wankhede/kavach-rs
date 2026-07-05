//! `.env`-file leak patterns: `source .env`, file-reading commands, and search.
use super::util::is_names_only;
use crate::gates::env_guard_dotenv::{
    detect_env_filename, is_safe_downstream, targets_dotenv_file,
};
use crate::gates::env_guard_grep_prefix::is_public_prefix_grep;
use crate::gates::env_guard_source_extract::{extract_post_source_command, has_source_builtin};

/// `source .env [&& downstream]` — loading is ALLOWED. Sourcing puts values in the
/// child process env, NOT in stdout/transcript; only a subsequent PRINT leaks them.
/// So bare `source .env` (and `set -a; . ./.env; set +a`) passes; a `&&`-chained
/// downstream is blocked ONLY when that downstream itself prints a value
/// (`echo $X`/`printenv`/`env`/raw shell — `is_safe_downstream` encodes the policy).
/// The leak-readers (`cat`/`bat`/`head`/`tail` on .env, `echo $VAR`, `printenv`,
/// `env|grep`) remain blocked by their own sibling patterns, untouched.
/// SOURCE: keyway.sh/articles/ai-coding-agents-secrets-security — "the leak is the
/// PRINT, not the load". See decision.engine.env-guard-source-load-allowed.
pub(crate) fn check_source(lc: &str, command: &str) -> Option<String> {
    if !(has_source_builtin(lc) && lc.contains(".env")) {
        return None;
    }
    let downstream = extract_post_source_command(command)?;
    if is_safe_downstream(&downstream) {
        return None;
    }
    let env_file = detect_env_filename(lc);
    Some(format!(
        "[SECRET_CONSUME] `source {env_file} && {downstream}` PRINTS a secret value into \
         context — sourcing itself is fine, the leak is the print. NOT a hand-back. DO THE \
         TASK: drop the printer; consume the value in-process (a runtime `.sh` that loads \
         `.env` then runs the op, or a rust-script via `dotenvy` + `std::env::var`) and emit \
         ONLY a pass/fail receipt — never the value. Run it yourself -> retry."
    ))
}

/// `cat`/`bat`/`head`/`tail`/`less`/`more`/`strings` reading a `.env` file.
pub(crate) fn check_dotenv_read(lc: &str) -> Option<String> {
    const READ_CMDS: &[&str] = &[
        "cat ", "bat ", "head ", "tail ", "less ", "more ", "strings ",
    ];
    if !(targets_dotenv_file(lc) && READ_CMDS.iter().any(|c| lc.starts_with(c))) {
        return None;
    }
    Some(
        "[SECRET_CONSUME] would leak every .env value into context. NOT a hand-back. \
         DO THE TASK: WRITE a runtime Rust script (or `cargo run`-able bin / `rust-script`) \
         that calls `dotenvy::from_path(\".env\")` then `std::env::var(..)` INSIDE its own \
         process, performs the operation the task needs (migration / deletion / query / op), \
         and prints ONLY a pass/fail receipt — never the secret value. The value lives in the \
         child process, never in your context. If a required key is absent after the load, \
         print `{\"ok\":false,\"missing\":[\"KEY\"]}` and file a card naming KEY. \
         (TypeScript with strict types only if Rust cannot reach the engine.) Run it yourself -> retry."
            .into(),
    )
}

/// `grep`/`rg` on a `.env` file — `PUBLIC_`-prefixed and names-only patterns exempt.
pub(crate) fn check_dotenv_grep(lc: &str) -> Option<String> {
    let uses_search = lc.contains("grep") || lc.starts_with("rg ") || lc.contains(" rg ");
    if !(targets_dotenv_file(lc) && uses_search && !is_names_only(lc)) || is_public_prefix_grep(lc)
    {
        return None;
    }
    Some(
        "[SECRET_CONSUME] `grep`/`rg` on .env prints secret values. NOT a hand-back. \
         DO THE TASK: if you need a value, consume it inside a runtime script \
         (`dotenvy` + `std::env::var`) that runs the operation and prints ONLY a receipt — \
         never the value. To inspect only NAMES: `rg -o '^[A-Z][A-Z0-9_]*' .env | sort`. \
         To read only public vars: `rg '^PUBLIC_' .env`. Run it yourself -> retry."
            .into(),
    )
}
