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
            "BLOCKED: `source {env_file} && {downstream}` — downstream command may expose values. \
             Use a safe runner (sqlx, cargo, kavach, bun run, make) that takes env vars \
             without printing them."
        ));
    }
    Some(format!(
        "BLOCKED: bare `source {env_file}` with no downstream command loads secrets into context. \
         Use `source {env_file} && <cmd>` with a safe runner, or list names only with: \
         `rg -o '^[A-Z][A-Z0-9_]*' {env_file} | sort` (toolbelt: rg is 5-13x faster than awk)."
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
        "BLOCKED: Reading a .env file via Bash exposes all secret values. \
         Use the Read tool instead (it reads .env files directly). \
         Or `grep '^PUBLIC_' .env` to extract only public (non-sensitive) variables."
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
        "BLOCKED: `grep`/`rg` on .env file exposes secret values. \
         Use `rg '^PUBLIC_' .env` to read only public (non-sensitive) vars (toolbelt: rg), \
         or use the Read tool to read .env files directly."
            .into(),
    )
}
