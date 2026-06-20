//! `.env`-file leak patterns: `source .env`, file-reading commands, and search.
use super::util::is_names_only;
use crate::gates::env_guard_dotenv::{
    detect_env_filename, is_safe_downstream, targets_dotenv_file,
};
use crate::gates::env_guard_grep_prefix::is_public_prefix_grep;
use crate::gates::env_guard_source_extract::{extract_post_source_command, has_source_builtin};

/// `source .env [&& downstream]` — loads values; safe runners exempt.
pub(crate) fn check_source(lc: &str, command: &str) -> Option<String> {
    if !(has_source_builtin(lc) && lc.contains(".env")) {
        return None;
    }
    let env_file = detect_env_filename(lc);
    if let Some(downstream) = extract_post_source_command(command) {
        if is_safe_downstream(&downstream) {
            return None;
        }
        return Some(format!(
            "BLOCKED (`source {env_file} && {downstream}` may print values). NOT a hand-back. \
             DO THE TASK: fold `{downstream}` into a runtime script that reads the env in-process \
             (`dotenvy::from_path(\"{env_file}\")` + `std::env::var`), runs it, and emits ONLY a \
             pass/fail receipt — never the value. Run it yourself; do NOT hand back to the user."
        ));
    }
    Some(format!(
        "BLOCKED (bare `source {env_file}` loads secrets into context). NOT a hand-back. \
         DO THE TASK: WRITE a runtime Rust script that loads `{env_file}` via dotenvy INSIDE its \
         own process, performs the needed operation, and prints ONLY a receipt — never the value. \
         To see only names (no values): `rg -o '^[A-Z][A-Z0-9_]*' {env_file} | sort`."
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
        "BLOCKED (would leak every .env value into context). This is NOT a hand-back. \
         DO THE TASK: WRITE a runtime Rust script (or `cargo run`-able bin / `rust-script`) \
         that calls `dotenvy::from_path(\".env\")` then `std::env::var(..)` INSIDE its own \
         process, performs the operation the task needs (migration / deletion / query / op), \
         and prints ONLY a pass/fail receipt — never the secret value. The value lives in the \
         child process, never in your context. If a required key is absent after the load, \
         print `{\"ok\":false,\"missing\":[\"KEY\"]}` and file a card naming KEY. \
         (TypeScript with strict types only if Rust cannot reach the engine.) Do NOT ask the user to run it."
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
        "BLOCKED (`grep`/`rg` on .env prints secret values). NOT a hand-back. \
         DO THE TASK: if you need a value, consume it inside a runtime script \
         (`dotenvy` + `std::env::var`) that runs the operation and prints ONLY a receipt — \
         never the value. To inspect only NAMES: `rg -o '^[A-Z][A-Z0-9_]*' .env | sort`. \
         To read only public vars: `rg '^PUBLIC_' .env`. Do NOT defer to the user."
            .into(),
    )
}
